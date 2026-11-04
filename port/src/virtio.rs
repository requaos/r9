//! port::virtio — the from-scratch, unsafe-free virtio 1.x modern transport
//! (D-01/D-02): the PCI capability model (§4.1.4), the §3.1.1
//! feature-negotiation state machine with the D-03 zero-offload accept
//! mask, and the split-virtqueue ring index model with explicit,
//! test-verified barrier placement (§2.7.13/§2.7.14 — D-04's host half).
//! virtio-net is the FIRST consumer (the modern, non-transitional PCI
//! device 0x1041 — the transitional 0x1000 is rejected at PCI discovery,
//! before this module's negotiation ever runs); nothing in the transport
//! is net-specific beyond the 12-byte header model, so virtio-blk/rng plug
//! into the same machinery later.
//!
//! Everything is pure safe Rust over indices plus two narrow volatile
//! seams: [`CommonCfg`] (the common/device config registers) and
//! [`RingMem`] (descriptor/avail/used ring memory + the notify register).
//! The arch side implements both over the io.rs volatile floor in 07-06;
//! the crate-root `virtio_tests` module drives the state machines over
//! Vec-backed mocks that record operation ORDER, so the barrier placement
//! is proven by test, not prose (ordering and negotiation are where this
//! domain's bugs live — Pitfalls N5/N6).
//!
//! Ring barrier discipline (the normative contract):
//! - post: descriptor writes → avail-entry write → `fence(Ordering::Release)`
//!   → avail-idx store → `fence(Ordering::Release)` → used-flags read →
//!   notify write;
//! - retire: used-idx read → `fence(Ordering::Acquire)` →
//!   descriptor-content reads.
//! `fence(Release)` lowers to `dmb ish` and `fence(Acquire)` to `dmb ishld`
//! on the pinned aarch64 toolchain (RESEARCH-verified codegen) — fences are
//! safe core intrinsics, so the module keeps zero unsafe and zero asm!.

#![forbid(unsafe_code)]

use core::sync::atomic::Ordering;

use crate::Result;

// ---------------------------------------------------------------------------
// PCI capability model (virtio-v1.3 §4.1.4)
// ---------------------------------------------------------------------------

/// PCI status register offset and its capabilities-list bit.
const PCI_OFF_STATUS: usize = 0x06;
const PCI_STATUS_CAP_LIST: u8 = 0x10;
/// The capability-list head pointer (config offset 0x34).
const PCI_OFF_CAP_LIST: usize = 0x34;
/// PCI_CAP_ID_VNDR — the vendor-specific capability id carrying virtio.
const PCI_CAP_ID_VNDR: u8 = 0x09;

// Field offsets inside struct virtio_pci_cap (§4.1.4).
const CAP_OFF_NEXT: usize = 1;
const CAP_OFF_LEN: usize = 2;
const CAP_OFF_CFG_TYPE: usize = 3;
const CAP_OFF_BAR: usize = 4;
const CAP_OFF_OFFSET: usize = 8;
const CAP_OFF_LENGTH: usize = 12;
const CAP_OFF_NOTIFY_MULT: usize = 16;
const CAP_BASE_LEN: usize = 16;
const CAP_NOTIFY_LEN: usize = 20;
/// Walk bound — a list that never terminates is a device fault, not a loop.
const MAX_CAPS: usize = 64;

// cfg_type values (§4.1.4.1).  Unknown/reserved values are skipped, and a
// larger cap_len is accepted (the driver reads only the fields it knows).
const CFG_TYPE_COMMON: u8 = 1;
const CFG_TYPE_NOTIFY: u8 = 2;
const CFG_TYPE_ISR: u8 = 3;
const CFG_TYPE_DEVICE: u8 = 4;

/// One capability's (bar, offset, length) triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapTriple {
    pub bar: u8,
    pub offset: u32,
    pub length: u32,
}

/// The typed capabilities a modern virtio PCI device presents, plus the
/// notify capability's notify_off_multiplier (§4.1.4.4: notify address =
/// BAR(notify.bar) base + notify.offset + queue_notify_off × multiplier).
/// `device` is optional in the transport (virtio-net always carries it —
/// the MAC lives there).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    pub common: CapTriple,
    pub notify: CapTriple,
    pub isr: CapTriple,
    pub device: Option<CapTriple>,
    pub notify_off_multiplier: u32,
}

/// The PCI config-space read seam — ECAM volatile reads arch-side, a byte
/// slice in the host tests.
pub trait PciCfg {
    /// The config-space extent in bytes (for truncation checks).
    fn len(&self) -> usize;
    fn read_u8(&self, off: usize) -> u8;
    fn read_u32(&self, off: usize) -> u32;
}

impl Caps {
    /// Walk the capability list collecting the vendor-0x09 virtio caps
    /// (§4.1.4.1: unknown cfg_type values are skipped; a larger cap_len is
    /// accepted).  A truncated or non-terminating list errors loudly, as
    /// does a missing common/notify/ISR triple.
    pub fn parse(cfg: &impl PciCfg) -> Result<Caps> {
        let _ = cfg;
        // RED stub — GREEN walks the list.  The zeroed Ok shape keeps every
        // behavior test failing on its assertions (a stub Err would satisfy
        // the error-path tests trivially).
        Ok(Caps {
            common: CapTriple { bar: 0, offset: 0, length: 0 },
            notify: CapTriple { bar: 0, offset: 0, length: 0 },
            isr: CapTriple { bar: 0, offset: 0, length: 0 },
            device: None,
            notify_off_multiplier: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// Common-config registers (§4.1.4.3) + the §3.1.1 negotiation machine
// ---------------------------------------------------------------------------

// Common-cfg offsets from the COMMON_CFG cap's (bar, offset).
pub const CFG_DEVICE_FEATURE_SELECT: usize = 0x00;
pub const CFG_DEVICE_FEATURE: usize = 0x04;
pub const CFG_DRIVER_FEATURE_SELECT: usize = 0x08;
pub const CFG_DRIVER_FEATURE: usize = 0x0C;
pub const CFG_DEVICE_STATUS: usize = 0x14;
pub const CFG_QUEUE_SELECT: usize = 0x16;
pub const CFG_QUEUE_SIZE: usize = 0x18;
pub const CFG_QUEUE_ENABLE: usize = 0x1C;
pub const CFG_QUEUE_NOTIFY_OFF: usize = 0x1E;
pub const CFG_QUEUE_DESC: usize = 0x20;
pub const CFG_QUEUE_DRIVER: usize = 0x28;
pub const CFG_QUEUE_DEVICE: usize = 0x30;

// Device status bits (§3.1).
pub const STATUS_ACKNOWLEDGE: u8 = 1;
pub const STATUS_DRIVER: u8 = 2;
pub const STATUS_DRIVER_OK: u8 = 4;
pub const STATUS_FEATURES_OK: u8 = 8;
pub const STATUS_FAILED: u8 = 128;

// Feature bits (§6 + §5.1.3).  The D-03 zero-offload accept mask is
// MAC | STATUS | VERSION_1 and NOTHING else — MRG_RXBUF(15), CSUM(0),
// EVENT_IDX(29), CTRL_VQ(17) and friends are never accepted, not even
// transiently to make something work (Pitfall N6); the negotiated mask
// printed at bind is the audit line.
pub const VIRTIO_NET_F_MAC: u32 = 5;
pub const VIRTIO_NET_F_STATUS: u32 = 16;
pub const VIRTIO_F_VERSION_1: u32 = 32;

/// Low feature window (bits 0..31) accept mask: MAC | STATUS.
pub const ACCEPT_LO: u32 = (1 << VIRTIO_NET_F_MAC) | (1 << VIRTIO_NET_F_STATUS);
/// High feature window (bits 32..63) accept mask: VERSION_1 only.
pub const ACCEPT_HI: u32 = 1 << (VIRTIO_F_VERSION_1 - 32);

/// Bound on the post-reset status read-back spin (§4.1.4.3.2: the driver
/// MUST NOT stop reading until the status reads 0 — a device that never
/// clears is broken, not slow, so the spin is bounded and loud).
const RESET_SPINS: u32 = 1_000_000;

/// The volatile common-config register seam (§4.1.3.1 access discipline:
/// width-matched accesses; 64-bit fields are written as two independent
/// 32-bit halves, low first).  Also the right seam for the DEVICE_CFG
/// region (the virtio-net MAC at offset 0) — same discipline, other cap.
pub trait CommonCfg {
    fn read8(&self, off: usize) -> u8;
    fn read16(&self, off: usize) -> u16;
    fn read32(&self, off: usize) -> u32;
    fn write8(&self, off: usize, v: u8);
    fn write16(&self, off: usize, v: u16);
    fn write32(&self, off: usize, v: u32);
}

/// The negotiated feature subset — the bind-time audit print consumes this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureMask {
    pub lo: u32,
    pub hi: u32,
}

impl FeatureMask {
    pub fn has_mac(&self) -> bool {
        self.lo & (1 << VIRTIO_NET_F_MAC) != 0
    }
    pub fn has_status(&self) -> bool {
        self.lo & (1 << VIRTIO_NET_F_STATUS) != 0
    }
    pub fn has_version_1(&self) -> bool {
        self.hi & (1 << (VIRTIO_F_VERSION_1 - 32)) != 0
    }
}

/// The §3.1.1 init sequence through FEATURES_OK: reset (write 0, wait for
/// the read-back 0), ACKNOWLEDGE, DRIVER, the two-window feature read, the
/// accepted-subset write (exactly MAC|STATUS low + VERSION_1 high, D-03),
/// FEATURES_OK with the re-read check.  Queue setup and DRIVER_OK are the
/// caller's later steps (queue_enable LAST; DRIVER_OK before any notify).
/// A device without VERSION_1 is rejected loudly and left FAILED (D-03).
pub fn negotiate(cfg: &impl CommonCfg) -> Result<FeatureMask> {
    let _ = cfg;
    Ok(FeatureMask { lo: 0, hi: 0 }) // RED stub — GREEN runs the machine
}

/// Read a queue's size (0 = unavailable).  qemu's virtio-net queues are
/// 256; accepted as-offered, never downsized (the plan's flagged
/// assumption — a size beyond MAX_QSIZE fails Ring::new loudly).
pub fn queue_size(cfg: &impl CommonCfg, queue: u16) -> u16 {
    let _ = (cfg, queue);
    0 // RED stub
}

/// Read a queue's notify offset (multiplied by notify_off_multiplier for
/// the notify address — §4.1.4.4).
pub fn queue_notify_off(cfg: &impl CommonCfg, queue: u16) -> u16 {
    let _ = (cfg, queue);
    0 // RED stub
}

/// Program the three queue area physical addresses (descriptor / avail /
/// used) as two 32-bit halves each (§4.1.3.1).  queue_enable stays
/// untouched here — the caller writes it LAST (§4.1.4.3.2).
pub fn set_queue_addresses(cfg: &impl CommonCfg, queue: u16, desc_pa: u64, avail_pa: u64, used_pa: u64) {
    let _ = (cfg, queue, desc_pa, avail_pa, used_pa); // RED stub
}

/// Enable a queue.  §4.1.4.3.2: every other virtqueue field MUST be
/// configured before this write; never write 0.
pub fn enable_queue(cfg: &impl CommonCfg, queue: u16) {
    let _ = (cfg, queue); // RED stub
}

/// DRIVER_OK — the device is live; notifications may follow (never before).
pub fn set_driver_ok(cfg: &impl CommonCfg) {
    let _ = cfg; // RED stub
}

// ---------------------------------------------------------------------------
// Split-virtqueue ring index model (§2.7) — the allocator.rs index/arena
// idiom re-expressed: side-table metadata + typed u16 indices, no intrusive
// pointers, raw memory only through the RingMem seam.
// ---------------------------------------------------------------------------

// Descriptor flags (§2.7.5.1).
pub const DESC_F_NEXT: u16 = 1;
pub const DESC_F_WRITE: u16 = 2;

// Ring layout (§2.7's table: desc 16-byte aligned, avail 2, used 4).
const DESC_STRIDE: usize = 16;
const DESC_OFF_ADDR: usize = 0;
const DESC_OFF_LEN: usize = 8;
const DESC_OFF_FLAGS: usize = 12;
const DESC_OFF_NEXT: usize = 14;
const AVAIL_OFF_FLAGS: usize = 0;
const AVAIL_OFF_IDX: usize = 2;
const AVAIL_OFF_RING: usize = 4;
const USED_OFF_FLAGS: usize = 0;
const USED_OFF_IDX: usize = 2;
const USED_OFF_RING: usize = 4;
const USED_ELEM_STRIDE: usize = 8;

/// used-ring flags bit: the device asks the driver not to notify
/// (§2.7.10.1 — the only suppression input; EVENT_IDX is never negotiated).
const USED_F_NO_NOTIFY: u16 = 1;

/// The queue-size ceiling the index model supports.  qemu's virtio-net
/// queues are 256 (accepted as-offered, never downsized — the plan's
/// EDGE-PROBE flagged assumption); a larger offer fails loudly at
/// construction rather than truncating the side tables.
pub const MAX_QSIZE: u16 = 256;
const MAX_SLOTS: usize = (MAX_QSIZE / 2) as usize;

/// The raw ring-memory volatile seam: the arch side implements this over
/// io.rs wrappers on the ring's pagealloc pages (mapped Normal at
/// KZERO+PA — never a device mapping); the host tests implement it over a
/// Vec arena recording op order.  All accesses are volatile by contract.
pub trait RingMem {
    fn read_u16(&self, off: usize) -> u16;
    fn read_u32(&self, off: usize) -> u32;
    fn read_u64(&self, off: usize) -> u64;
    fn write_u16(&self, off: usize, v: u16);
    fn write_u32(&self, off: usize, v: u32);
    fn write_u64(&self, off: usize, v: u64);

    /// The §2.7.13/§2.7.14 barrier at the ring's ordering points.  The
    /// default lowers to core's fence — Release → `dmb ish`, Acquire →
    /// `dmb ishld` on the pinned aarch64 toolchain (RESEARCH-verified
    /// codegen).  The host-test mock overrides to record the fence's
    /// position in the op log.
    fn fence(&self, order: Ordering) {
        core::sync::atomic::fence(order);
    }
}

/// Queue direction: RX descriptors are device-writable (WRITE on both
/// chain members), TX descriptors are device-read (neither) — the
/// ethervirtio.c ring semantics re-expressed in the index model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Rx,
    Tx,
}

/// A retired used-ring buffer: the owning slot plus the device-written
/// byte count.  RX lengths INCLUDE the 12-byte header (the frame length is
/// `len - VIRTIO_NET_HDR_LEN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsedBuf {
    pub slot: u16,
    pub len: u32,
}

/// The descriptor index pool: a fixed-size free stack of u16 descriptor
/// indices (allocator.rs's index/arena idiom — side-table metadata, typed
/// indices, no intrusive links in the descriptor area itself).
pub(crate) struct DescPool {
    free: [u16; MAX_QSIZE as usize],
    /// free[0..len] is the free stack (top at len-1; a fresh pool pops
    /// ascending from 0).
    len: u16,
}

impl DescPool {
    pub(crate) fn new(qsize: u16) -> DescPool {
        let _ = qsize;
        DescPool { free: [0; MAX_QSIZE as usize], len: 0 } // RED stub
    }

    pub(crate) fn alloc(&mut self) -> Option<u16> {
        Some(0) // RED stub — GREEN pops the stack
    }

    /// Recycle an index.  No teardown path exists this era — the free half
    /// is exercised by the host tests and by future queue-reset work.
    #[allow(dead_code)]
    pub(crate) fn free(&mut self, idx: u16) {
        let _ = idx; // RED stub
    }
}

/// The split-virtqueue ring state machine — pure index/arena logic over
/// the RingMem volatile seam; zero MMIO knowledge, zero unsafe.
pub struct Ring<M: RingMem> {
    desc: M,
    avail: M,
    used: M,
    qsize: u16,
    dir: Dir,
    pool: DescPool,
    /// Per-slot descriptor chain head (nslots entries) — the avail
    /// pre-link table: a slot's chain head never changes, so posting is a
    /// fixed-value avail-entry write plus the idx advance.
    slot_head: [u16; MAX_SLOTS],
    /// Driver-side ownership: the slot holds a posted buffer (the C
    /// driver's `blocks[i] != nil`).
    posted: [bool; MAX_SLOTS],
    /// What we last stored to avail->idx (u16 wrap at 2^16).
    avail_idx: u16,
    /// Our used-drain position.
    last_used: u16,
    /// Used-len validation bounds (V5 — the device is untrusted input):
    /// RX queues carry (12, buffer capacity); TX (0, u32::MAX) — TX lens
    /// are informational only.
    used_len_min: u32,
    used_len_max: u32,
}

impl<M: RingMem> Ring<M> {
    /// Build the ring over three pre-carved memory regions (desc 16×qsize
    /// bytes, avail 6+2×qsize, used 6+8×qsize — §2.7's alignments are the
    /// arch side's problem).  Assembles the two-descriptor chains (header
    /// desc NEXT-flagged to the payload desc; RX both flagged WRITE, TX
    /// neither) and pre-links every avail entry to its slot's chain head.
    /// `hdr_pa` is the shared per-queue header page's physical address
    /// (its 12-byte content is never consumed by the driver — MRG_RXBUF is
    /// off, so num_buffers is always 1 — so one header per queue suffices,
    /// ethervirtio.c's shape).
    pub fn new(
        desc: M,
        avail: M,
        used: M,
        qsize: u16,
        dir: Dir,
        hdr_pa: u64,
        rx_buf_cap: u32,
    ) -> Result<Ring<M>> {
        // RED stub — GREEN validates qsize and assembles the chains.
        let _ = hdr_pa;
        let (used_len_min, used_len_max) = match dir {
            Dir::Rx => (VIRTIO_NET_HDR_LEN as u32, rx_buf_cap),
            Dir::Tx => (0, u32::MAX),
        };
        Ok(Ring {
            desc,
            avail,
            used,
            qsize,
            dir,
            pool: DescPool::new(qsize),
            slot_head: [0; MAX_SLOTS],
            posted: [false; MAX_SLOTS],
            avail_idx: 0,
            last_used: 0,
            used_len_min,
            used_len_max,
        })
    }

    fn nslots(&self) -> usize {
        (self.qsize / 2) as usize
    }

    fn qmask(&self) -> u16 {
        self.qsize - 1
    }

    fn slot_mask(&self) -> u16 {
        (self.qsize >> 1) - 1
    }

    /// The queue size accepted from the device at setup.
    pub fn qsize(&self) -> u16 {
        self.qsize
    }

    /// The driver's avail position (telemetry + tests).
    pub fn avail_idx(&self) -> u16 {
        self.avail_idx
    }

    /// The driver's used-drain position (telemetry + tests).
    pub fn last_used(&self) -> u16 {
        self.last_used
    }

    /// Post a TX frame: fill the slot's payload descriptor, then the
    /// §2.7.13 publication sequence — descriptor writes → avail-entry →
    /// fence(Release) → avail-idx store → fence(Release) → used-flags
    /// read → notify.  A full ring is a loud error, never an alias —
    /// retire used entries first.
    pub fn tx_post(
        &mut self,
        payload_pa: u64,
        payload_len: u32,
        notify: &M,
        notify_off: usize,
        queue_index: u16,
    ) -> Result<u16> {
        let _ = (payload_pa, payload_len, notify, notify_off, queue_index);
        Ok(0) // RED stub — GREEN runs the barrier sandwich
    }

    /// RX replenish-until-caught-up, then ONE notify (ethervirtio.c's
    /// rxproc loop): per slot — payload desc write → avail-entry →
    /// fence(Release) → idx advance; after the loop — fence(Release) →
    /// used-flags → notify.  `alloc` yields (payload_pa, capacity) for a
    /// fresh buffer or None when the pool is empty.  Returns the number
    /// of buffers offered.
    pub fn rx_replenish(
        &mut self,
        alloc: impl FnMut() -> Option<(u64, u32)>,
        notify: &M,
        notify_off: usize,
        queue_index: u16,
    ) -> Result<u32> {
        let _ = (alloc, notify, notify_off, queue_index);
        Ok(0) // RED stub
    }

    /// Drain one used-ring entry: None when caught up
    /// (last_used == used.idx); otherwise the §2.7.14 acquire side — idx
    /// read → fence(Acquire) → entry reads — then the V5 validation (id <
    /// qsize and even; len within the queue's bounds).  A failed
    /// validation is a device fault: surfaced as Err with the bogus entry
    /// consumed and the slot left posted (the caller retires the device,
    /// not the ring).
    pub fn next_used(&mut self) -> Option<Result<UsedBuf>> {
        let _ = self;
        None // RED stub
    }
}

// ---------------------------------------------------------------------------
// The virtio-net packet header (§5.1.6)
// ---------------------------------------------------------------------------

/// The header is 12 bytes whenever VIRTIO_F_VERSION_1 is negotiated —
/// regardless of MRG_RXBUF (§5.1.6.1 + qemu's virtio_net_set_mrg_rx_bufs;
/// the 10-byte form is legacy-only and D-03 forbids the legacy path).
pub const VIRTIO_NET_HDR_LEN: usize = 12;

/// The modeled header (§5.1.6.1); little-endian on the wire.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VirtioNetHdr {
    pub flags: u8,
    pub gso_type: u8,
    pub hdr_len: u16,
    pub gso_size: u16,
    pub csum_start: u16,
    pub csum_offset: u16,
    pub num_buffers: u16,
}

const _: () = assert!(core::mem::size_of::<VirtioNetHdr>() == VIRTIO_NET_HDR_LEN);

impl VirtioNetHdr {
    /// The TX header under the D-03 zero-offload mask: flags, gso_type and
    /// num_buffers zeroed (§5.1.6.2.1 — no CSUM, no GSO, no merging).
    pub fn tx() -> VirtioNetHdr {
        VirtioNetHdr::default()
    }

    /// The little-endian wire bytes (the arch side volatile-writes them
    /// into the queue's header page).
    pub fn to_le_bytes(&self) -> [u8; VIRTIO_NET_HDR_LEN] {
        let _ = self;
        [0u8; VIRTIO_NET_HDR_LEN] // RED stub — GREEN serializes the fields
    }
}
