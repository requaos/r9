//! The virtio transport's test module: homed at the CRATE root beside
//! allocator_tests.rs (the crate-root placement precedent — the mock
//! arenas need alloc/std, while port::virtio itself carries
//! #[forbid(unsafe_code)]).  The mock RingMem is a Vec-backed arena whose
//! every volatile read/write and fence lands in a shared op log, so the
//! barrier-placement tests assert operation ORDER, not just outcomes
//! (D-04's host half: ordering and negotiation are where this domain's
//! bugs live — Pitfalls N5/N6).

use core::sync::atomic::Ordering;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::virtio::{
    self, CapTriple, Caps, CommonCfg, DESC_F_NEXT, DESC_F_WRITE, DescPool, Dir, PciCfg, Ring,
    RingMem, STATUS_ACKNOWLEDGE, STATUS_DRIVER, STATUS_DRIVER_OK, STATUS_FEATURES_OK, UsedBuf,
    VIRTIO_NET_HDR_LEN, VirtioNetHdr,
};

/// One recorded volatile operation (order in the log IS the assertion
/// surface for the barrier sandwich and the §3.1.1 step order).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Op {
    /// Region, offset, width in bytes.
    Read(&'static str, usize, u8),
    /// Region, offset, value, width in bytes.
    Write(&'static str, usize, u64, u8),
    Fence(Ordering),
}

/// Assert the expected ops appear in the log in the given relative order
/// (other ops may interleave — the assertion is about ORDER, not exact
/// trace equality).
fn assert_subsequence(log: &[Op], expected: &[Op]) {
    let mut rest = log.iter();
    for want in expected {
        assert!(
            rest.any(|got| got == want),
            "missing or out-of-order op: {want:?}\nlog tail: {log:#?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Mock ring memory (RingMem over a Vec arena + the op log)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct MockMem {
    name: &'static str,
    buf: Rc<RefCell<Vec<u8>>>,
    log: Rc<RefCell<Vec<Op>>>,
}

impl MockMem {
    fn named(name: &'static str, size: usize, log: &Rc<RefCell<Vec<Op>>>) -> MockMem {
        MockMem { name, buf: Rc::new(RefCell::new(vec![0u8; size])), log: log.clone() }
    }

    /// A quiet arena for the long-running wrap test (its own log, never
    /// inspected — 66000 posts would grow the shared log to ~500k entries).
    fn quiet(name: &'static str, size: usize) -> MockMem {
        MockMem {
            name,
            buf: Rc::new(RefCell::new(vec![0u8; size])),
            log: Rc::new(RefCell::new(Vec::new())),
        }
    }

    // Device-side pokes and driver-side peeks: direct buffer access, NOT
    // logged (the device does not write through the driver's op log).
    fn device_write_u16(&self, off: usize, v: u16) {
        self.buf.borrow_mut()[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn device_write_u32(&self, off: usize, v: u32) {
        self.buf.borrow_mut()[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn peek_u16(&self, off: usize) -> u16 {
        u16::from_le_bytes(self.buf.borrow()[off..off + 2].try_into().unwrap())
    }
    fn peek_u32(&self, off: usize) -> u32 {
        u32::from_le_bytes(self.buf.borrow()[off..off + 4].try_into().unwrap())
    }
    fn peek_u64(&self, off: usize) -> u64 {
        u64::from_le_bytes(self.buf.borrow()[off..off + 8].try_into().unwrap())
    }
}

impl RingMem for MockMem {
    fn read_u16(&self, off: usize) -> u16 {
        self.log.borrow_mut().push(Op::Read(self.name, off, 2));
        self.peek_u16(off)
    }
    fn read_u32(&self, off: usize) -> u32 {
        self.log.borrow_mut().push(Op::Read(self.name, off, 4));
        self.peek_u32(off)
    }
    fn read_u64(&self, off: usize) -> u64 {
        self.log.borrow_mut().push(Op::Read(self.name, off, 8));
        self.peek_u64(off)
    }
    fn write_u16(&self, off: usize, v: u16) {
        self.log.borrow_mut().push(Op::Write(self.name, off, u64::from(v), 2));
        self.device_write_u16(off, v);
    }
    fn write_u32(&self, off: usize, v: u32) {
        self.log.borrow_mut().push(Op::Write(self.name, off, u64::from(v), 4));
        self.device_write_u32(off, v);
    }
    fn write_u64(&self, off: usize, v: u64) {
        self.log.borrow_mut().push(Op::Write(self.name, off, v, 8));
        self.buf.borrow_mut()[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }
    fn fence(&self, order: Ordering) {
        self.log.borrow_mut().push(Op::Fence(order));
        core::sync::atomic::fence(order);
    }
}

// ---------------------------------------------------------------------------
// Mock common-cfg (register file + feature windows + reset/FEATURES_OK scripts)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct MockCommonCfg {
    regs: Rc<RefCell<Vec<u8>>>,
    log: Rc<RefCell<Vec<Op>>>,
    /// device_feature windows [bits 0..31, bits 32..63].
    features: [u32; 2],
    /// queue_size per queue_select (rx 0, tx 1) — qemu's 256 default.
    queue_sizes: [u16; 2],
    /// queue_notify_off per queue (qemu maps queue N → N).
    notify_offs: [u16; 2],
    /// Reset script: a status write of 0 arms this many nonzero read-backs
    /// before the status clears (the device mid-reset).
    reset_script: u32,
    reset_reads_left: Rc<Cell<u32>>,
    /// When set, status reads never reflect FEATURES_OK (the device
    /// refusing the driver's accepted subset — §3.1.1 step 6).
    suppress_features_ok: Rc<Cell<bool>>,
}

impl MockCommonCfg {
    fn new(log: &Rc<RefCell<Vec<Op>>>) -> MockCommonCfg {
        MockCommonCfg {
            regs: Rc::new(RefCell::new(vec![0u8; 0x40])),
            log: log.clone(),
            features: [u32::MAX, u32::MAX],
            queue_sizes: [256, 256],
            notify_offs: [0, 1],
            reset_script: 0,
            reset_reads_left: Rc::new(Cell::new(0)),
            suppress_features_ok: Rc::new(Cell::new(false)),
        }
    }

    /// The raw status byte (no scripting — the driver's final register state).
    fn status(&self) -> u8 {
        self.regs.borrow()[virtio::CFG_DEVICE_STATUS]
    }

    fn selected_queue(&self) -> usize {
        (self.peek16(virtio::CFG_QUEUE_SELECT) & 1) as usize
    }

    fn peek16(&self, off: usize) -> u16 {
        u16::from_le_bytes(self.regs.borrow()[off..off + 2].try_into().unwrap())
    }
    fn peek32(&self, off: usize) -> u32 {
        u32::from_le_bytes(self.regs.borrow()[off..off + 4].try_into().unwrap())
    }
}

impl CommonCfg for MockCommonCfg {
    fn read8(&self, off: usize) -> u8 {
        self.log.borrow_mut().push(Op::Read("common", off, 1));
        let mut v = self.regs.borrow()[off];
        if off == virtio::CFG_DEVICE_STATUS {
            let left = self.reset_reads_left.get();
            if left > 0 {
                self.reset_reads_left.set(left - 1);
                return 0x55; // still resetting
            }
            if self.suppress_features_ok.get() {
                v &= !virtio::STATUS_FEATURES_OK;
            }
        }
        v
    }
    fn read16(&self, off: usize) -> u16 {
        self.log.borrow_mut().push(Op::Read("common", off, 2));
        match off {
            virtio::CFG_QUEUE_SIZE => self.queue_sizes[self.selected_queue()],
            virtio::CFG_QUEUE_NOTIFY_OFF => self.notify_offs[self.selected_queue()],
            _ => self.peek16(off),
        }
    }
    fn read32(&self, off: usize) -> u32 {
        self.log.borrow_mut().push(Op::Read("common", off, 4));
        if off == virtio::CFG_DEVICE_FEATURE {
            let sel = self.peek32(virtio::CFG_DEVICE_FEATURE_SELECT) as usize;
            return self.features[sel & 1];
        }
        self.peek32(off)
    }
    fn write8(&self, off: usize, v: u8) {
        self.log.borrow_mut().push(Op::Write("common", off, u64::from(v), 1));
        if off == virtio::CFG_DEVICE_STATUS && v == 0 {
            self.reset_reads_left.set(self.reset_script);
        }
        self.regs.borrow_mut()[off] = v;
    }
    fn write16(&self, off: usize, v: u16) {
        self.log.borrow_mut().push(Op::Write("common", off, u64::from(v), 2));
        self.regs.borrow_mut()[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn write32(&self, off: usize, v: u32) {
        self.log.borrow_mut().push(Op::Write("common", off, u64::from(v), 4));
        self.regs.borrow_mut()[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
}

// ---------------------------------------------------------------------------
// Mock PCI config space (a byte slice)
// ---------------------------------------------------------------------------

struct SliceCfg(Vec<u8>);

impl PciCfg for SliceCfg {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn read_u8(&self, off: usize) -> u8 {
        self.0[off]
    }
    fn read_u32(&self, off: usize) -> u32 {
        u32::from_le_bytes(self.0[off..off + 4].try_into().unwrap())
    }
}

/// A vendor capability list in a 256-byte config space: common → notify →
/// ISR → device → a cfg_type-9 (VENDOR) cap that must be skipped → PCI_CFG
/// (walked past) — qemu 11.1.2's shape plus the §4.1.4.1 skip path.
fn caps_fixture() -> Vec<u8> {
    let mut b = vec![0u8; 0x100];
    b[0x06] = 0x10; // status: capabilities list present
    b[0x34] = 0x40; // list head
    fn put(b: &mut [u8], at: usize, next: usize, cfg_type: u8, bar: u8, offset: u32, length: u32) {
        b[at] = 0x09; // PCI_CAP_ID_VNDR
        b[at + 1] = next as u8;
        b[at + 2] = 16; // cap_len (notify overwrites below)
        b[at + 3] = cfg_type;
        b[at + 4] = bar;
        b[at + 8..at + 12].copy_from_slice(&offset.to_le_bytes());
        b[at + 12..at + 16].copy_from_slice(&length.to_le_bytes());
    }
    put(&mut b, 0x40, 0x60, 1, 4, 0x0000, 0x1000); // common
    put(&mut b, 0x60, 0x80, 2, 4, 0x3000, 0x1000); // notify
    b[0x62] = 20; // notify cap_len covers the multiplier
    b[0x70..0x74].copy_from_slice(&4u32.to_le_bytes()); // notify_off_multiplier at +16
    put(&mut b, 0x80, 0xA0, 3, 4, 0x1000, 0x1000); // ISR
    put(&mut b, 0xA0, 0xC0, 4, 4, 0x2000, 0x1000); // device
    put(&mut b, 0xC0, 0xE0, 9, 0, 0, 0); // unknown cfg_type — skipped per §4.1.4.1
    put(&mut b, 0xE0, 0, 5, 0, 0, 0); // PCI_CFG — walked past
    b
}

// ---------------------------------------------------------------------------
// Ring fixture
// ---------------------------------------------------------------------------

const HDR_PA: u64 = 0x1000_0000;
const RX_BUF_CAP: u32 = 4096;

struct Rig {
    ring: Ring<MockMem>,
    desc: MockMem,
    avail: MockMem,
    used: MockMem,
    notify: MockMem,
    log: Rc<RefCell<Vec<Op>>>,
}

fn rig_with(qsize: u16, dir: Dir, quiet: bool) -> Rig {
    let log = Rc::new(RefCell::new(Vec::new()));
    let (desc, avail, used, notify) = if quiet {
        (
            MockMem::quiet("desc", 16 * qsize as usize),
            MockMem::quiet("avail", 6 + 2 * qsize as usize),
            MockMem::quiet("used", 6 + 8 * qsize as usize),
            MockMem::quiet("notify", 8),
        )
    } else {
        (
            MockMem::named("desc", 16 * qsize as usize, &log),
            MockMem::named("avail", 6 + 2 * qsize as usize, &log),
            MockMem::named("used", 6 + 8 * qsize as usize, &log),
            MockMem::named("notify", 8, &log),
        )
    };
    let ring = Ring::new(desc.clone(), avail.clone(), used.clone(), qsize, dir, HDR_PA, RX_BUF_CAP)
        .expect("ring setup");
    Rig { ring, desc, avail, used, notify, log }
}

fn rig(qsize: u16, dir: Dir) -> Rig {
    rig_with(qsize, dir, false)
}

/// The device side of the used ring: write the entry at the DEVICE's own
/// used position (read back from the buffer — the device's cursor is
/// independent of the driver's last_used) and advance used.idx.  Direct
/// buffer pokes — the device does not log through the driver's op log.
fn device_complete(rig: &Rig, id: u32, len: u32) {
    let dev_pos = rig.used.peek_u16(2);
    let eoff = 4 + (dev_pos & (rig.ring.qsize() - 1)) as usize * 8;
    rig.used.device_write_u32(eoff, id);
    rig.used.device_write_u32(eoff + 4, len);
    rig.used.device_write_u16(2, dev_pos.wrapping_add(1));
}

/// Drain one entry, asserting an entry is pending (the RED phase fails on
/// this assertion, not on an unwrap panic).
fn drain_one(rig: &mut Rig) -> crate::Result<UsedBuf> {
    let used = rig.ring.next_used();
    assert!(used.is_some(), "an entry is pending");
    used.unwrap()
}

// ---------------------------------------------------------------------------
// Capability-model tests
// ---------------------------------------------------------------------------

#[test]
fn caps_parse_yields_the_typed_triples_and_notify_multiplier() {
    let cfg = SliceCfg(caps_fixture());
    let caps = Caps::parse(&cfg).expect("the five-cap fixture parses");
    assert_eq!(caps.common, CapTriple { bar: 4, offset: 0x0000, length: 0x1000 });
    assert_eq!(caps.notify, CapTriple { bar: 4, offset: 0x3000, length: 0x1000 });
    assert_eq!(caps.isr, CapTriple { bar: 4, offset: 0x1000, length: 0x1000 });
    assert_eq!(caps.device, Some(CapTriple { bar: 4, offset: 0x2000, length: 0x1000 }));
    assert_eq!(caps.notify_off_multiplier, 4, "the notify cap's extra data");
    // The fixture's cfg_type-9 and PCI_CFG caps were walked past without
    // disturbing the four typed triples (§4.1.4.1).
}

#[test]
fn caps_parse_truncated_list_errors_loudly() {
    let mut b = caps_fixture();
    b.truncate(0x48); // the common cap at 0x40 needs 16 bytes: 0x50 > 0x48
    let err = Caps::parse(&SliceCfg(b)).expect_err("a truncated cap errors loudly");
    assert!(err.contains("truncated"), "the error names the truncation: {err}");
}

#[test]
fn caps_parse_missing_list_errors_loudly() {
    let mut b = caps_fixture();
    b[0x06] = 0; // status: no capabilities list
    assert!(Caps::parse(&SliceCfg(b)).is_err());
}

// ---------------------------------------------------------------------------
// Negotiation tests (§3.1.1, the D-03 mask)
// ---------------------------------------------------------------------------

#[test]
fn negotiate_accepts_exactly_mac_status_and_version1() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let cfg = MockCommonCfg::new(&log); // offers every feature bit in both windows
    let mask = virtio::negotiate(&cfg).expect("VERSION_1 offered → negotiate succeeds");
    // The D-03 zero-offload mask, exactly: MAC(5) | STATUS(16) | VERSION_1(32)
    // (VERSION_1 is window-1 bit 0).  MRG_RXBUF(15), CSUM(0), EVENT_IDX(29)
    // were all offered (all-ones windows) and NOT accepted.
    assert_eq!(mask.lo, (1u32 << 5) | (1u32 << 16), "low window: MAC|STATUS only");
    assert_eq!(mask.hi, 1u32, "high window: VERSION_1 only");
    assert!(mask.has_mac() && mask.has_status() && mask.has_version_1());
    // Not even transiently more (Pitfall N6): the only values ever written
    // to driver_feature are the two accepted windows.
    let writes: Vec<u64> = log
        .borrow()
        .iter()
        .filter_map(|op| match op {
            Op::Write("common", 0x0C, v, 4) => Some(*v),
            _ => None,
        })
        .collect();
    assert_eq!(writes, vec![u64::from((1u32 << 5) | (1u32 << 16)), 1]);
    assert_eq!(
        cfg.status(),
        virtio::STATUS_ACKNOWLEDGE | virtio::STATUS_DRIVER | virtio::STATUS_FEATURES_OK
    );
}

#[test]
fn negotiate_rejects_loudly_without_version1_and_marks_failed() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut cfg = MockCommonCfg::new(&log);
    cfg.features = [u32::MAX, !1]; // every bit except VERSION_1 (window-1 bit 0)
    let err = virtio::negotiate(&cfg).expect_err("no VERSION_1 → loud reject");
    assert!(err.contains("VERSION_1"), "the error names the missing feature: {err}");
    assert_eq!(
        cfg.status() & virtio::STATUS_FAILED,
        virtio::STATUS_FAILED,
        "the device is left FAILED"
    );
}

#[test]
fn negotiate_rejects_when_features_ok_is_not_reflected() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let cfg = MockCommonCfg::new(&log);
    cfg.suppress_features_ok.set(true);
    let err = virtio::negotiate(&cfg).expect_err("FEATURES_OK not reflected → reject");
    assert!(err.contains("FEATURES_OK"), "the error names the check: {err}");
    assert_eq!(cfg.status() & virtio::STATUS_FAILED, virtio::STATUS_FAILED);
}

#[test]
fn negotiate_step_order_is_the_3_1_1_sequence_and_reset_waits_for_readback() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut cfg = MockCommonCfg::new(&log);
    cfg.reset_script = 2; // two nonzero read-backs before the status clears
    virtio::negotiate(&cfg).expect("negotiate succeeds");
    assert_subsequence(
        &log.borrow(),
        &[
            Op::Write("common", 0x14, 0, 1), // reset
            Op::Read("common", 0x14, 1),     // read-back: still resetting
            Op::Read("common", 0x14, 1),     // read-back: still resetting
            Op::Read("common", 0x14, 1),     // read-back: 0 → proceed
            Op::Write("common", 0x14, 1, 1), // ACKNOWLEDGE
            Op::Write("common", 0x14, 3, 1), // + DRIVER
            Op::Write("common", 0x00, 0, 4), // device_feature_select = 0
            Op::Read("common", 0x04, 4),     // device_feature bits 0..31
            Op::Write("common", 0x00, 1, 4), // select = 1
            Op::Read("common", 0x04, 4),     // bits 32..63
            Op::Write("common", 0x08, 0, 4), // driver_feature_select = 0
            Op::Write("common", 0x0C, u64::from((1u32 << 5) | (1u32 << 16)), 4),
            Op::Write("common", 0x08, 1, 4),
            Op::Write("common", 0x0C, 1, 4),
            Op::Write("common", 0x14, 11, 1), // + FEATURES_OK
            Op::Read("common", 0x14, 1),      // the re-read check
        ],
    );
}

#[test]
fn queue_setup_writes_addresses_then_enable_last_and_driver_ok() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let cfg = MockCommonCfg::new(&log);
    assert_eq!(virtio::queue_size(&cfg, 0), 256);
    assert_eq!(virtio::queue_notify_off(&cfg, 1), 1);
    virtio::set_queue_addresses(&cfg, 0, 0x1000, 0x2000, 0x3000);
    virtio::enable_queue(&cfg, 0);
    virtio::set_driver_ok(&cfg);
    // §4.1.4.3.2: every other queue field is configured BEFORE queue_enable;
    // §3.1.1: DRIVER_OK before any notify.
    assert_subsequence(
        &log.borrow(),
        &[
            Op::Write("common", 0x16, 0, 2),      // queue_select = 0
            Op::Write("common", 0x20, 0x1000, 4), // queue_desc lo
            Op::Write("common", 0x24, 0, 4),      // queue_desc hi
            Op::Write("common", 0x28, 0x2000, 4), // queue_driver (avail)
            Op::Write("common", 0x2C, 0, 4),
            Op::Write("common", 0x30, 0x3000, 4), // queue_device (used)
            Op::Write("common", 0x34, 0, 4),
            Op::Write("common", 0x16, 0, 2), // select again (for enable)
            Op::Write("common", 0x1C, 1, 2), // queue_enable = 1 — LAST
            Op::Write("common", 0x14, 15, 1), // DRIVER_OK (ACK|DRIVER|FEATURES_OK|DRIVER_OK)
        ],
    );
    assert_eq!(
        cfg.status(),
        STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK
    );
}

// ---------------------------------------------------------------------------
// Ring state-machine tests
// ---------------------------------------------------------------------------

#[test]
fn descriptor_pool_alloc_free_by_index() {
    let mut pool = DescPool::new(8);
    assert_eq!(pool.alloc(), Some(0));
    assert_eq!(pool.alloc(), Some(1));
    pool.free(1);
    assert_eq!(pool.alloc(), Some(1), "a freed index is recycled");
    let mut seen = vec![0u16, 1];
    while let Some(i) = pool.alloc() {
        seen.push(i);
    }
    seen.sort_unstable();
    assert_eq!(seen, vec![0, 1, 2, 3, 4, 5, 6, 7], "every index exactly once");
    assert_eq!(pool.alloc(), None, "exhaustion is loud (None), never a wrap");
}

#[test]
fn chain_assembly_flags_rx_device_writable_tx_device_read() {
    let rx = rig(8, Dir::Rx);
    // Header desc 0: NEXT|WRITE → payload desc 1 (both device-writable on RX).
    assert_eq!(rx.desc.peek_u16(0 * 16 + 12), DESC_F_NEXT | DESC_F_WRITE);
    assert_eq!(rx.desc.peek_u16(0 * 16 + 14), 1);
    assert_eq!(rx.desc.peek_u64(0), HDR_PA);
    assert_eq!(rx.desc.peek_u32(8), VIRTIO_NET_HDR_LEN as u32);
    assert_eq!(rx.desc.peek_u16(1 * 16 + 12), DESC_F_WRITE);
    assert_eq!(rx.desc.peek_u16(1 * 16 + 14), 0);
    let tx = rig(8, Dir::Tx);
    assert_eq!(tx.desc.peek_u16(0 * 16 + 12), DESC_F_NEXT);
    assert_eq!(tx.desc.peek_u16(1 * 16 + 12), 0, "TX descriptors are device-read (no WRITE)");
}

#[test]
fn avail_ring_prelinked_slots_permanently_name_chain_heads() {
    let r = rig(8, Dir::Tx); // 4 slots: heads 0, 2, 4, 6
    for p in 0..8usize {
        let want = (2 * (p % 4)) as u16;
        assert_eq!(
            r.avail.peek_u16(4 + 2 * p),
            want,
            "avail entry {p} names its slot's chain head"
        );
    }
}

#[test]
fn tx_post_fills_payload_advances_idx_and_ring_full_is_loud() {
    let mut r = rig(8, Dir::Tx);
    assert_eq!(r.ring.tx_post(0x2000_0000, 60, &r.notify, 0, 1), Ok(0));
    assert_eq!(r.desc.peek_u64(1 * 16), 0x2000_0000, "payload desc addr");
    assert_eq!(r.desc.peek_u32(1 * 16 + 8), 60, "payload desc len");
    assert_eq!(r.avail.peek_u16(2), 1, "avail idx advanced");
    assert_eq!(r.ring.tx_post(0x2000_1000, 60, &r.notify, 0, 1), Ok(1));
    assert_eq!(r.ring.tx_post(0x2000_2000, 60, &r.notify, 0, 1), Ok(2));
    assert_eq!(r.ring.tx_post(0x2000_3000, 60, &r.notify, 0, 1), Ok(3));
    let err = r.ring.tx_post(0x2000_4000, 60, &r.notify, 0, 1);
    assert!(err.is_err(), "a full ring is a loud error, never an alias: {err:?}");
}

#[test]
fn tx_post_barrier_sandwich_is_the_2_7_13_order() {
    let mut r = rig(8, Dir::Tx);
    r.ring.tx_post(0x2000_0000, 60, &r.notify, 0, 1).unwrap();
    assert_subsequence(
        &r.log.borrow(),
        &[
            Op::Write("desc", 16, 0x2000_0000, 8), // payload desc addr
            Op::Write("desc", 24, 60, 4),          // payload desc len
            Op::Write("avail", 4, 0, 2),           // avail entry = chain head 0
            Op::Fence(Ordering::Release),          // device sees table+entry
            Op::Write("avail", 2, 1, 2),           // avail idx = 1
            Op::Fence(Ordering::Release),          // idx visible before flags read
            Op::Read("used", 0, 2),                // used flags (suppression check)
            Op::Write("notify", 0, 1, 2),          // notify queue 1
        ],
    );
}

#[test]
fn tx_post_suppresses_notify_when_device_sets_no_notify() {
    let mut r = rig(8, Dir::Tx);
    r.used.device_write_u16(0, 1); // VIRTQ_USED_F_NO_NOTIFY
    r.ring.tx_post(0x2000_0000, 60, &r.notify, 0, 1).unwrap();
    assert_eq!(r.avail.peek_u16(2), 1, "the post itself went through (idx advanced)");
    let notified = r.log.borrow().iter().any(|op| matches!(op, Op::Write("notify", ..)));
    assert!(!notified, "the notify write is suppressed by used.flags");
}

#[test]
fn used_drain_is_acquire_ordered_and_recovers_the_slot() {
    let mut r = rig(8, Dir::Tx);
    assert_eq!(r.ring.tx_post(0x2000_0000, 60, &r.notify, 0, 1), Ok(0));
    device_complete(&r, 0, 72); // the device echoes the chain-head id 0
    let used = drain_one(&mut r).expect("the entry validates");
    assert_eq!(used, UsedBuf { slot: 0, len: 72 });
    assert!(r.ring.next_used().is_none(), "drained");
    // Slot recovery: the slot is chosen by the avail POSITION, so the
    // retired slot 0 is postable again when the position comes back around
    // (posts 1..3 take the fresh slots first).
    assert_eq!(r.ring.tx_post(0x2000_1000, 60, &r.notify, 0, 1), Ok(1));
    assert_eq!(r.ring.tx_post(0x2000_2000, 60, &r.notify, 0, 1), Ok(2));
    assert_eq!(r.ring.tx_post(0x2000_3000, 60, &r.notify, 0, 1), Ok(3));
    assert_eq!(
        r.ring.tx_post(0x2000_8000, 60, &r.notify, 0, 1),
        Ok(0),
        "the retired slot is postable again (no RingFull on a recovered slot)"
    );
    // §2.7.14: used-idx read → fence(Acquire) → entry content reads.
    assert_subsequence(
        &r.log.borrow(),
        &[
            Op::Write("notify", 0, 1, 2), // the first post's notify
            Op::Read("used", 2, 2),       // drain: used idx
            Op::Fence(Ordering::Acquire), // trust contents only after
            Op::Read("used", 4, 4),       // entry id
            Op::Read("used", 8, 4),       // entry len
        ],
    );
}

#[test]
fn used_drain_rejects_out_of_range_and_odd_ids() {
    let mut r = rig(8, Dir::Tx);
    r.ring.tx_post(0x2000_0000, 60, &r.notify, 0, 1).unwrap();
    device_complete(&r, 8, 72); // id == qsize — out of range
    assert!(drain_one(&mut r).is_err(), "id >= qsize is a device fault");

    let mut r = rig(8, Dir::Tx);
    r.ring.tx_post(0x2000_0000, 60, &r.notify, 0, 1).unwrap();
    device_complete(&r, 3, 72); // odd id — chain heads are even
    assert!(drain_one(&mut r).is_err(), "an odd id would alias the wrong slot");
}

#[test]
fn used_drain_rejects_lens_outside_the_queue_bounds() {
    // RX: len must cover the 12-byte header and fit the buffer capacity.
    let mut r = rig(8, Dir::Rx);
    let mut once = Some((0x3000_0000u64, RX_BUF_CAP));
    let offered = r.ring.rx_replenish(|_| once.take(), &r.notify, 0, 0).unwrap();
    assert_eq!(offered.offered, 1);
    device_complete(&r, 0, 11); // shorter than the virtio-net header
    assert!(drain_one(&mut r).is_err(), "len < 12 is a device fault");

    let mut r = rig(8, Dir::Rx);
    let mut once = Some((0x3000_0000u64, RX_BUF_CAP));
    r.ring.rx_replenish(|_| once.take(), &r.notify, 0, 0).unwrap();
    device_complete(&r, 0, RX_BUF_CAP + 1);
    assert!(drain_one(&mut r).is_err(), "len beyond the buffer capacity is a device fault");

    let mut r = rig(8, Dir::Rx);
    let mut once = Some((0x3000_0000u64, RX_BUF_CAP));
    r.ring.rx_replenish(|_| once.take(), &r.notify, 0, 0).unwrap();
    device_complete(&r, 0, 72); // 12-byte header + 60-byte frame
    assert_eq!(drain_one(&mut r).expect("in-bounds len validates"), UsedBuf { slot: 0, len: 72 });
}

#[test]
fn rx_replenish_posts_until_caught_up_then_one_notify() {
    let mut r = rig(8, Dir::Rx);
    let mut nbuf = 0u32;
    let offered = r
        .ring
        .rx_replenish(
            |_| {
                nbuf += 1;
                Some((0x3000_0000 + u64::from(nbuf) * 4096, RX_BUF_CAP))
            },
            &r.notify,
            0,
            0,
        )
        .unwrap();
    assert_eq!(offered.offered, 4, "all four slots pre-posted");
    assert!(offered.notified, "no suppression -> the burst's one notify");
    assert_eq!(r.avail.peek_u16(2), 4, "avail idx after the burst");
    for s in 0..4usize {
        let payload = 2 * s + 1;
        assert_eq!(r.desc.peek_u64(payload * 16), 0x3000_0000 + (s as u64 + 1) * 4096);
        assert_eq!(r.desc.peek_u32(payload * 16 + 8), RX_BUF_CAP);
    }
    let notifies = r.log.borrow().iter().filter(|op| matches!(op, Op::Write("notify", ..))).count();
    assert_eq!(notifies, 1, "ONE notify for the whole burst");

    // The device consumes two; replenish offers exactly two more (+1 notify).
    device_complete(&r, 0, 72);
    device_complete(&r, 2, 72);
    assert!(drain_one(&mut r).is_ok());
    assert!(drain_one(&mut r).is_ok());
    let offered = r.ring.rx_replenish(|_| Some((0x3000_8000, RX_BUF_CAP)), &r.notify, 0, 0).unwrap();
    assert_eq!(offered.offered, 2, "replenish refills only the freed slots");
    assert!(offered.notified);
    let notifies = r.log.borrow().iter().filter(|op| matches!(op, Op::Write("notify", ..))).count();
    assert_eq!(notifies, 2);
}

#[test]
fn rx_replenish_reports_the_slot_and_suppression() {
    // The alloc closure learns the slot being posted (the arch side binds
    // each slot's buffer page there), and the Replenish outcome carries
    // the notify truth for the nnote accounting.
    let mut r = rig(8, Dir::Rx);
    let mut slots = std::vec::Vec::new();
    let out = r
        .ring
        .rx_replenish(
            |s| {
                slots.push(s);
                Some((0x3100_0000 + u64::from(s) * 4096, RX_BUF_CAP))
            },
            &r.notify,
            0,
            0,
        )
        .unwrap();
    assert_eq!(slots.as_slice(), &[0, 1, 2, 3], "alloc learns the slot being posted");
    assert_eq!(out, virtio::Replenish { offered: 4, notified: true });

    // With the device asking for no notifications, a refill reports
    // !notified (and the ring really writes none).
    device_complete(&r, 0, 72);
    assert!(drain_one(&mut r).is_ok());
    r.used.device_write_u16(0, 1); // USED_F_NO_NOTIFY
    let out = r.ring.rx_replenish(|_| Some((0x3200_0000, RX_BUF_CAP)), &r.notify, 0, 0).unwrap();
    assert_eq!(out, virtio::Replenish { offered: 1, notified: false });
    let notifies = r.log.borrow().iter().filter(|op| matches!(op, Op::Write("notify", ..))).count();
    assert_eq!(notifies, 1, "only the first burst notified");
}

#[test]
fn indices_wrap_at_u16_without_losing_slot_identity() {
    let mut r = rig_with(256, Dir::Tx, true); // 128 slots, quiet arenas
    let total = 66000u32; // > 2^16: avail/used idx wrap mid-test
    for i in 0..total {
        let want_slot = (i % 128) as u16;
        let slot = r.ring.tx_post(0x2000_0000, 60, &r.notify, 0, 1).unwrap();
        assert_eq!(slot, want_slot);
        device_complete(&r, u32::from(2 * want_slot), 72);
        let used = drain_one(&mut r).expect("the entry validates");
        assert_eq!(used.slot, want_slot);
    }
    // 66000 mod 65536 = 464 — both counters wrapped at 2^16 and kept going.
    assert_eq!(r.ring.avail_idx(), total as u16, "u16 wrap at 2^16");
    assert_eq!(r.ring.last_used(), total as u16);
}

#[test]
fn ring_rejects_bad_queue_sizes() {
    let mk = |qsize: u16| {
        Ring::new(
            MockMem::quiet("desc", 16 * 256),
            MockMem::quiet("avail", 6 + 2 * 256),
            MockMem::quiet("used", 6 + 8 * 256),
            qsize,
            Dir::Tx,
            HDR_PA,
            RX_BUF_CAP,
        )
    };
    assert!(mk(0).is_err(), "size 0");
    assert!(mk(3).is_err(), "not a power of two");
    assert!(mk(512).is_err(), "beyond MAX_QSIZE fails loudly, never truncated");
    assert!(mk(256).is_ok(), "qemu's 256 is accepted as-offered");
}

#[test]
fn virtio_net_hdr_is_12_bytes_with_a_zeroed_tx_form() {
    assert_eq!(VIRTIO_NET_HDR_LEN, 12, "VERSION_1 forces the 12-byte header (§5.1.6.1)");
    assert_eq!(core::mem::size_of::<VirtioNetHdr>(), VIRTIO_NET_HDR_LEN);
    let tx = VirtioNetHdr::tx();
    assert_eq!((tx.flags, tx.gso_type, tx.num_buffers), (0, 0, 0), "§5.1.6.2.1");
    assert_eq!(tx.to_le_bytes(), [0u8; 12]);
    // Layout pin: hdr_len at bytes 2..4 LE, num_buffers at 10..12 LE.
    let hdr = VirtioNetHdr { hdr_len: 0x1234, num_buffers: 1, ..VirtioNetHdr::default() };
    let b = hdr.to_le_bytes();
    assert_eq!(&b[2..4], &[0x34, 0x12]);
    assert_eq!(&b[10..12], &[1, 0]);
}
