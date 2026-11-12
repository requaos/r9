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
//!
//! ## The seam 07-06 implements (boot-time contract)
//!
//! The arch side supplies three volatile implementations over the io.rs
//! floor:
//! - [`PciCfg`] over the mapped ECAM window (for [`Caps::parse`]);
//! - [`CommonCfg`] over the mapped BAR4 common-config region (the RegBlock
//!   idiom) — the same seam shape serves the device-config region and the
//!   ISR-byte reader (an 8-bit read at the ISR cap's (bar, offset);
//!   read-to-ack per §4.1.4.5 — the read deasserts the level line);
//! - [`RingMem`] over each queue's descriptor/avail/used areas and the
//!   notify register — pagealloc pages mapped NORMAL at KZERO+PA (never
//!   device mappings); the default `fence` method stands (core's fences —
//!   `dmb ish` / `dmb ishld` on aarch64).
//!
//! Boot call order (§4.1.4.3.2 / §3.1.1 — pinned by the host tests):
//! `Caps::parse` → `negotiate` → per queue (rx 0, tx 1): allocate + map
//! the ring pages → `Ring::new` over the VA side → `queue_size` (accepted
//! as-offered, never downsized) → `set_queue_addresses` with the physical
//! addresses → `queue_notify_off` → `enable_queue` LAST → `set_driver_ok`
//! → `rx_replenish` pre-posts the RX buffers (the first notify write —
//! never before DRIVER_OK).  The negotiated `FeatureMask` print at bind is
//! the D-03 audit line.

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
        if cfg.read_u8(PCI_OFF_STATUS) & PCI_STATUS_CAP_LIST == 0 {
            return Err("virtio: device presents no capability list");
        }
        let mut common = None;
        let mut notify = None;
        let mut isr = None;
        let mut device = None;
        let mut notify_off_multiplier = 0u32;
        let mut p = cfg.read_u8(PCI_OFF_CAP_LIST) as usize;
        let mut walked = 0usize;
        while p != 0 {
            walked += 1;
            if walked > MAX_CAPS {
                return Err("virtio: capability list does not terminate");
            }
            if p + CAP_BASE_LEN > cfg.len() {
                return Err("virtio: truncated capability list");
            }
            let next = cfg.read_u8(p + CAP_OFF_NEXT) as usize;
            if cfg.read_u8(p) == PCI_CAP_ID_VNDR {
                let cap_len = cfg.read_u8(p + CAP_OFF_LEN) as usize;
                if cap_len < CAP_BASE_LEN {
                    return Err("virtio: vendor capability shorter than the base struct");
                }
                let triple = CapTriple {
                    bar: cfg.read_u8(p + CAP_OFF_BAR),
                    offset: cfg.read_u32(p + CAP_OFF_OFFSET),
                    length: cfg.read_u32(p + CAP_OFF_LENGTH),
                };
                match cfg.read_u8(p + CAP_OFF_CFG_TYPE) {
                    CFG_TYPE_COMMON => common = Some(triple),
                    CFG_TYPE_NOTIFY => {
                        if p + CAP_NOTIFY_LEN > cfg.len() {
                            return Err("virtio: truncated notify capability");
                        }
                        notify_off_multiplier = cfg.read_u32(p + CAP_OFF_NOTIFY_MULT);
                        notify = Some(triple);
                    }
                    CFG_TYPE_ISR => isr = Some(triple),
                    CFG_TYPE_DEVICE => device = Some(triple),
                    // PCI_CFG / SHARED_MEMORY / VENDOR / reserved: walked
                    // past per §4.1.4.1 (unknown cfg_type/bar are ignored).
                    _ => {}
                }
            }
            p = next;
        }
        match (common, notify, isr) {
            (Some(common), Some(notify), Some(isr)) => {
                Ok(Caps { common, notify, isr, device, notify_off_multiplier })
            }
            _ => Err("virtio: common/notify/ISR capabilities incomplete"),
        }
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
    // Step 1: reset — write 0, then wait for the read-back 0 (§4.1.4.3.2:
    // the driver MUST NOT stop reading until the status reads 0).
    cfg.write8(CFG_DEVICE_STATUS, 0);
    let mut spins = 0u32;
    while cfg.read8(CFG_DEVICE_STATUS) != 0 {
        spins += 1;
        if spins >= RESET_SPINS {
            return Err("virtio: device status never read back 0 after reset");
        }
    }
    // Steps 2-3: ACKNOWLEDGE, then DRIVER (the status set is cumulative).
    cfg.write8(CFG_DEVICE_STATUS, STATUS_ACKNOWLEDGE);
    cfg.write8(CFG_DEVICE_STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER);
    // Step 4: two-window feature read; write the accepted subset.
    cfg.write32(CFG_DEVICE_FEATURE_SELECT, 0);
    let offered_lo = cfg.read32(CFG_DEVICE_FEATURE);
    cfg.write32(CFG_DEVICE_FEATURE_SELECT, 1);
    let offered_hi = cfg.read32(CFG_DEVICE_FEATURE);
    if offered_hi & ACCEPT_HI == 0 {
        // §6: "A driver MAY fail to operate further if VIRTIO_F_VERSION_1
        // is not offered" — D-03 takes that branch, loudly.
        cfg.write8(CFG_DEVICE_STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FAILED);
        return Err("virtio: VIRTIO_F_VERSION_1 not offered — legacy/transitional rejected");
    }
    let accepted = FeatureMask { lo: offered_lo & ACCEPT_LO, hi: offered_hi & ACCEPT_HI };
    cfg.write32(CFG_DRIVER_FEATURE_SELECT, 0);
    cfg.write32(CFG_DRIVER_FEATURE, accepted.lo);
    cfg.write32(CFG_DRIVER_FEATURE_SELECT, 1);
    cfg.write32(CFG_DRIVER_FEATURE, accepted.hi);
    // Steps 5-6: FEATURES_OK, then the re-read check.
    cfg.write8(CFG_DEVICE_STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK);
    if cfg.read8(CFG_DEVICE_STATUS) & STATUS_FEATURES_OK == 0 {
        cfg.write8(CFG_DEVICE_STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FAILED);
        return Err("virtio: device rejected the feature subset (FEATURES_OK not reflected)");
    }
    Ok(accepted)
}

/// Read a queue's size (0 = unavailable).  qemu's virtio-net queues are
/// 256; accepted as-offered, never downsized (the plan's flagged
/// assumption — a size beyond MAX_QSIZE fails Ring::new loudly).
pub fn queue_size(cfg: &impl CommonCfg, queue: u16) -> u16 {
    cfg.write16(CFG_QUEUE_SELECT, queue);
    cfg.read16(CFG_QUEUE_SIZE)
}

/// Read a queue's notify offset (multiplied by notify_off_multiplier for
/// the notify address — §4.1.4.4).
pub fn queue_notify_off(cfg: &impl CommonCfg, queue: u16) -> u16 {
    cfg.write16(CFG_QUEUE_SELECT, queue);
    cfg.read16(CFG_QUEUE_NOTIFY_OFF)
}

/// Program the three queue area physical addresses (descriptor / avail /
/// used) as two 32-bit halves each (§4.1.3.1).  queue_enable stays
/// untouched here — the caller writes it LAST (§4.1.4.3.2).
pub fn set_queue_addresses(
    cfg: &impl CommonCfg,
    queue: u16,
    desc_pa: u64,
    avail_pa: u64,
    used_pa: u64,
) {
    cfg.write16(CFG_QUEUE_SELECT, queue);
    cfg.write32(CFG_QUEUE_DESC, desc_pa as u32);
    cfg.write32(CFG_QUEUE_DESC + 4, (desc_pa >> 32) as u32);
    cfg.write32(CFG_QUEUE_DRIVER, avail_pa as u32);
    cfg.write32(CFG_QUEUE_DRIVER + 4, (avail_pa >> 32) as u32);
    cfg.write32(CFG_QUEUE_DEVICE, used_pa as u32);
    cfg.write32(CFG_QUEUE_DEVICE + 4, (used_pa >> 32) as u32);
}

/// Enable a queue.  §4.1.4.3.2: every other virtqueue field MUST be
/// configured before this write; never write 0.
pub fn enable_queue(cfg: &impl CommonCfg, queue: u16) {
    cfg.write16(CFG_QUEUE_SELECT, queue);
    cfg.write16(CFG_QUEUE_ENABLE, 1);
}

/// DRIVER_OK — the device is live; notifications may follow (never before).
pub fn set_driver_ok(cfg: &impl CommonCfg) {
    cfg.write8(
        CFG_DEVICE_STATUS,
        STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK,
    );
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
        let mut free = [0u16; MAX_QSIZE as usize];
        // Pop order runs ascending: free[0] = qsize-1 … free[qsize-1] = 0.
        let mut i = 0u16;
        while i < qsize {
            free[(qsize - 1 - i) as usize] = i;
            i += 1;
        }
        DescPool { free, len: qsize }
    }

    pub(crate) fn alloc(&mut self) -> Option<u16> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.free[self.len as usize])
    }

    /// Recycle an index.  No teardown path exists this era — the free half
    /// is exercised by the host tests and by future queue-reset work.
    #[allow(dead_code)]
    pub(crate) fn free(&mut self, idx: u16) {
        self.free[self.len as usize] = idx;
        self.len += 1;
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
        if qsize < 2 || qsize > MAX_QSIZE || !qsize.is_power_of_two() {
            return Err("virtio: bad queue size (want a power of two in 2..=256)");
        }
        let (used_len_min, used_len_max) = match dir {
            Dir::Rx => (VIRTIO_NET_HDR_LEN as u32, rx_buf_cap),
            Dir::Tx => (0, u32::MAX),
        };
        let mut ring = Ring {
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
        };
        // The driver wants interrupts: leave Rnointerrupt clear in
        // avail.flags (ethervirtio.c's `q->avail->flags &= ~Rnointerrupt`).
        ring.avail.write_u16(AVAIL_OFF_FLAGS, 0);
        ring.setup_chains(hdr_pa);
        Ok(ring)
    }

    /// Assemble the two-descriptor chains and pre-link the avail ring.  A
    /// fresh pool pops ascending, so slot i gets chain head 2i —
    /// ethervirtio.c's static layout, reproduced organically.
    fn setup_chains(&mut self, hdr_pa: u64) {
        let nslots = self.nslots();
        let dir_flag = match self.dir {
            Dir::Rx => DESC_F_WRITE,
            Dir::Tx => 0,
        };
        for slot in 0..nslots {
            let head = self.pool.alloc().expect("virtio: fresh pool covers 2 descriptors per slot");
            let payload =
                self.pool.alloc().expect("virtio: fresh pool covers 2 descriptors per slot");
            let hb = head as usize * DESC_STRIDE;
            self.desc.write_u64(hb + DESC_OFF_ADDR, hdr_pa);
            self.desc.write_u32(hb + DESC_OFF_LEN, VIRTIO_NET_HDR_LEN as u32);
            self.desc.write_u16(hb + DESC_OFF_FLAGS, DESC_F_NEXT | dir_flag);
            self.desc.write_u16(hb + DESC_OFF_NEXT, payload);
            let pb = payload as usize * DESC_STRIDE;
            self.desc.write_u16(pb + DESC_OFF_FLAGS, dir_flag);
            self.desc.write_u16(pb + DESC_OFF_NEXT, 0);
            self.slot_head[slot] = head;
        }
        // Avail pre-linking: every avail position permanently names its
        // slot's chain head, so posting is the fixed-value entry write
        // plus the idx advance.
        for p in 0..self.qsize {
            let slot = (p as usize) & (nslots - 1);
            self.avail.write_u16(AVAIL_OFF_RING + 2 * p as usize, self.slot_head[slot]);
        }
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

    /// The DEVICE's used position (the ring's used.idx, read volatile) —
    /// the ifstats line's `used=` field (07-07's driver hook).  Reading it
    /// carries no ordering obligation on its own (telemetry only); the
    /// drain path's acquire fence is what content-trust requires.
    pub fn used_idx(&self) -> u16 {
        self.used.read_u16(USED_OFF_IDX)
    }

    /// The slot the next TX post would occupy, when it is free (None =
    /// the ring is full — the caller blocks until a retire frees one).
    /// Lets the TX caller bind the slot's payload buffer BEFORE posting
    /// (devether's write path copies the frame into the slot's page).
    pub fn tx_next_slot(&self) -> Option<u16> {
        if self.dir != Dir::Tx {
            return None;
        }
        let slot = (self.avail_idx & self.slot_mask()) as usize;
        if self.posted[slot] { None } else { Some(slot as u16) }
    }

    /// Post a TX frame: fill the slot's payload descriptor, then the
    /// §2.7.13 publication sequence — descriptor writes → avail-entry →
    /// fence(Release) → avail-idx store → fence(Release) → used-flags
    /// read → notify.  A full ring is a loud error, never an alias —
    /// retire used entries first.  The `Posted` return carries the notify
    /// truth so the caller's nnote accounting stays exact under
    /// suppression (the rx_replenish `Replenish` shape).
    pub fn tx_post(
        &mut self,
        payload_pa: u64,
        payload_len: u32,
        notify: &M,
        notify_off: usize,
        queue_index: u16,
    ) -> Result<Posted> {
        if self.dir != Dir::Tx {
            return Err("virtio: tx_post on an RX ring");
        }
        let slot = (self.avail_idx & self.slot_mask()) as usize;
        if self.posted[slot] {
            return Err("virtio: TX ring full — retire used entries first");
        }
        let payload = self.slot_head[slot] + 1;
        let pb = payload as usize * DESC_STRIDE;
        self.desc.write_u64(pb + DESC_OFF_ADDR, payload_pa);
        self.desc.write_u32(pb + DESC_OFF_LEN, payload_len);
        // The avail entry permanently names the slot's chain head (the
        // pre-linked value); the write is part of the §2.7.13 publication.
        let p = (self.avail_idx & self.qmask()) as usize;
        self.avail.write_u16(AVAIL_OFF_RING + 2 * p, self.slot_head[slot]);
        // §2.7.13 step 4: the device must see the descriptor table and the
        // avail ring entry before the idx update.
        self.avail.fence(Ordering::Release);
        self.avail_idx = self.avail_idx.wrapping_add(1);
        self.avail.write_u16(AVAIL_OFF_IDX, self.avail_idx);
        // §2.7.13.4.1: the idx must be visible before the suppression check.
        self.used.fence(Ordering::Release);
        let flags = self.used.read_u16(USED_OFF_FLAGS);
        let mut notified = false;
        if flags & USED_F_NO_NOTIFY == 0 {
            notify.write_u16(notify_off, queue_index);
            notified = true;
        }
        self.posted[slot] = true;
        Ok(Posted { slot: slot as u16, notified })
    }

    /// RX replenish-until-caught-up, then ONE notify (ethervirtio.c's
    /// rxproc loop): per slot — payload desc write → avail-entry →
    /// fence(Release) → idx advance; after the loop — fence(Release) →
    /// used-flags → notify.  `alloc` yields (payload_pa, capacity) for a
    /// fresh buffer FOR THE SLOT BEING POSTED (the arch side binds each
    /// slot's buffer page there), or None when the pool is empty.
    pub fn rx_replenish(
        &mut self,
        mut alloc: impl FnMut(u16) -> Option<(u64, u32)>,
        notify: &M,
        notify_off: usize,
        queue_index: u16,
    ) -> Result<Replenish> {
        if self.dir != Dir::Rx {
            return Err("virtio: rx_replenish on a TX ring");
        }
        let mut offered = 0u32;
        loop {
            let slot = (self.avail_idx & self.slot_mask()) as usize;
            if self.posted[slot] {
                break;
            }
            let Some((pa, cap)) = alloc(slot as u16) else {
                break;
            };
            let payload = self.slot_head[slot] + 1;
            let pb = payload as usize * DESC_STRIDE;
            self.desc.write_u64(pb + DESC_OFF_ADDR, pa);
            self.desc.write_u32(pb + DESC_OFF_LEN, cap);
            let p = (self.avail_idx & self.qmask()) as usize;
            self.avail.write_u16(AVAIL_OFF_RING + 2 * p, self.slot_head[slot]);
            self.avail.fence(Ordering::Release);
            self.avail_idx = self.avail_idx.wrapping_add(1);
            self.avail.write_u16(AVAIL_OFF_IDX, self.avail_idx);
            self.posted[slot] = true;
            offered += 1;
            // ethervirtio.c's replenish loop: post until caught up with the
            // device's used position, then ONE notify for the whole burst.
            if self.avail_idx == self.used.read_u16(USED_OFF_IDX) {
                break;
            }
        }
        let mut notified = false;
        if offered > 0 {
            self.used.fence(Ordering::Release);
            let flags = self.used.read_u16(USED_OFF_FLAGS);
            if flags & USED_F_NO_NOTIFY == 0 {
                notify.write_u16(notify_off, queue_index);
                notified = true;
            }
        }
        Ok(Replenish { offered, notified })
    }

    /// Drain one used-ring entry: None when caught up
    /// (last_used == used.idx); otherwise the §2.7.14 acquire side — idx
    /// read → fence(Acquire) → entry reads — then the V5 validation (id <
    /// qsize and even; len within the queue's bounds).  A failed
    /// validation is a device fault: surfaced as Err with the bogus entry
    /// consumed and the slot left posted (the caller retires the device,
    /// not the ring).
    pub fn next_used(&mut self) -> Option<Result<UsedBuf>> {
        let used_idx = self.used.read_u16(USED_OFF_IDX);
        if self.last_used == used_idx {
            return None;
        }
        // §2.7.14: the entry contents are trusted only after an acquire
        // barrier observing the idx advance.
        self.used.fence(Ordering::Acquire);
        let eoff = USED_OFF_RING + (self.last_used & self.qmask()) as usize * USED_ELEM_STRIDE;
        let id = self.used.read_u32(eoff);
        let len = self.used.read_u32(eoff + 4);
        self.last_used = self.last_used.wrapping_add(1);
        // V5: the device wrote id/len — validate before either indexes or
        // sizes anything (a device fault is a loud error, never an alias).
        if id >= u32::from(self.qsize) || id & 1 != 0 {
            return Some(Err("virtio: used-ring id out of range (device fault)"));
        }
        if len < self.used_len_min {
            return Some(Err("virtio: used-ring len below the minimum (device fault)"));
        }
        if len > self.used_len_max {
            return Some(Err("virtio: used-ring len beyond the buffer capacity (device fault)"));
        }
        // Slot recovery (ethervirtio.c): the used id is the chain head.
        let slot = ((id & u32::from(self.qmask())) >> 1) as usize;
        // WR-06 (17-03) slot liveness: a duplicated used id (a lying
        // device double-completing a chain head) reaches here with the
        // slot already recovered — retiring it again would push one
        // buffer page into the caller's free stack twice, aliasing two
        // live chains onto the same page.  Loud reject in the same
        // family as the checks above; never a panic (V5 posture).
        if !self.posted[slot] {
            return Some(Err("virtio: used-ring id for an unposted slot (device fault)"));
        }
        self.posted[slot] = false;
        Some(Ok(UsedBuf { slot: slot as u16, len }))
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
        let mut b = [0u8; VIRTIO_NET_HDR_LEN];
        b[0] = self.flags;
        b[1] = self.gso_type;
        b[2..4].copy_from_slice(&self.hdr_len.to_le_bytes());
        b[4..6].copy_from_slice(&self.gso_size.to_le_bytes());
        b[6..8].copy_from_slice(&self.csum_start.to_le_bytes());
        b[8..10].copy_from_slice(&self.csum_offset.to_le_bytes());
        b[10..12].copy_from_slice(&self.num_buffers.to_le_bytes());
        b
    }
}

// ---------------------------------------------------------------------------
// virtio_net — the transport's first consumer (D-02; the arch-side bind
// lands with it in 07-06): queue indices, ISR semantics, the RX
// frame-length rule, and the IRQ-accounting counters.  Pure data and
// small safe helpers — the arch side owns the volatile implementations
// and the publish-once device state.
// ---------------------------------------------------------------------------

/// §5.1.2 queue indices: queue 0 is receiveq1, queue 1 transmitq1.
/// (Queue 2 would be the control queue — CTRL_VQ is never negotiated
/// under D-03, so it does not exist.)
pub const VIRTIO_NET_QUEUE_RX: u16 = 0;
pub const VIRTIO_NET_QUEUE_TX: u16 = 1;

/// ISR status byte bits (§4.1.4.5): bit 0 = queue interrupt (used-ring
/// progress), bit 1 = device configuration change (virtio-net: link
/// status).  Reading the byte resets it to 0 and deasserts the interrupt
/// line — a handler's FIRST device touch is always this read.
pub const VIRTIO_ISR_QUEUE: u8 = 0x1;
pub const VIRTIO_ISR_CONFIG: u8 = 0x2;

/// RX frame length from a used-ring length: strip the 12-byte header
/// (§5.1.6 — the used len includes it).  Callers pass a len already
/// bounds-validated by `Ring::next_used` (≥ VIRTIO_NET_HDR_LEN), so the
/// subtraction cannot underflow.
pub const fn virtio_net_frame_len(used_len: u32) -> u32 {
    used_len - VIRTIO_NET_HDR_LEN as u32
}

/// The offset WITHIN an RX payload buffer at which the delivered frame
/// begins: ZERO — the frame begins at the payload descriptor's offset 0.
/// RX buffers are posted as two-descriptor chains
/// (setup_chains/rx_replenish): a 12-byte virtio_net_hdr descriptor
/// pointing at the queue's shared header page, NEXT-chained to the
/// payload descriptor.  Per §5.1.6.3.2 the device writes the header into
/// the FIRST descriptor and the frame into the SECOND at offset 0 — the
/// descriptor boundary is a hard I/O-vector boundary (qemu's iov_from_buf
/// fills desc-1's exactly 12 bytes before touching desc-2).  The used
/// length includes the header (virtio_net_frame_len strips it), but the
/// payload buffer never contains it.
pub const fn virtio_net_frame_offset() -> usize {
    0
}

/// Per-queue IRQ accounting (ethervirtio.c's nintr/nnote — NETDRV-04's
/// reconciliation surface; 07-07's ifstats file reads these).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct VirtioNetCounters {
    /// Interrupts that observed work, per queue (ethervirtio.c's nintr:
    /// one per interrupt per queue with pending used entries).
    pub nintr: [u32; 2],
    /// Notifications written, per queue (ethervirtio.c's nnote).
    pub nnote: [u32; 2],
}

/// The rx_replenish outcome: how many buffers were offered and whether
/// the closing notify was written (the caller's nnote accounting input —
/// suppression stays inside the ring, the count stays exact).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Replenish {
    pub offered: u32,
    pub notified: bool,
}

/// The tx_post outcome: the slot the frame occupies plus whether the
/// notify write happened (the caller's nnote accounting input — the same
/// suppression-exactness contract as `Replenish`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Posted {
    pub slot: u16,
    pub notified: bool,
}
