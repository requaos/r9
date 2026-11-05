//! The RX chain slice-arithmetic pins: homed at the CRATE root beside
//! allocator_tests.rs/virtio_tests.rs/devether_tests.rs (the crate-root
//! placement precedent — lib.rs's registration block; the Vec-backed
//! arena needs alloc/std while port::virtio keeps its
//! #[forbid(unsafe_code)]).
//!
//! These pins cover the Ring↔pci.rs seam that no prior gate touched:
//! the Ring host tests never simulated a device WRITING into the RX
//! chain, and the devether tests drive rx_ingress directly, bypassing
//! the arch side's slice construction — the reason every prior gate
//! asserted counters yet never delivery.  The drive is chain-faithful
//! per virtio §5.1.6.3.2: the device writes the 12-byte virtio_net_hdr
//! into the FIRST descriptor (the shared per-queue header page) and the
//! frame into the SECOND descriptor (the payload page) AT OFFSET 0 —
//! the descriptor boundary is a hard I/O-vector boundary (qemu's
//! iov_from_buf fills desc-1's exactly 12 bytes before touching desc-2).
//!
//! DESIGN NOTE — this module deliberately does NOT drive
//! netif::rx_ingress/pop_frame: netif::publish is once-per-process (the
//! is_none assert) and devether_tests' conv_close pin already consumes
//! that single publish in this test binary, so a second full-path drive
//! panics the suite.  The slice→rx_ingress→pop_frame leg is
//! host-covered by the devether_tests synthetic drive, and the composed
//! end-to-end confirmation is the guest UAT re-run.  What is pinned
//! here is the seam that was uncovered: the REAL Ring plus the REAL
//! arch-side slice arithmetic (virtio_net_frame_len /
//! virtio_net_frame_offset) against a simulated device write.

use std::cell::RefCell;
use std::rc::Rc;

use crate::virtio::{self, Dir, Replenish, Ring, RingMem, UsedBuf, VIRTIO_NET_HDR_LEN};

// Spec-fixed ring layout constants, local to this module so the
// byte-level pins cannot drift with virtio.rs's private constants:
// descriptor stride 16 with addr@0/len@8/flags@12/next@14 (§2.7.5).
const DESC_STRIDE: usize = 16; // §2.7.5
const DESC_OFF_ADDR: usize = 0; // §2.7.5
const DESC_OFF_LEN: usize = 8; // §2.7.5
const DESC_OFF_FLAGS: usize = 12; // §2.7.5
const DESC_OFF_NEXT: usize = 14; // §2.7.5
// Descriptor flags (§2.7.5.1).
const DESC_F_NEXT: u16 = 1; // §2.7.5.1: the chain continues in `next`
const DESC_F_WRITE: u16 = 2; // §2.7.5.1: the device writes this buffer
// Used ring: idx at offset 2, ring entries at 4, 8-byte elements
// (§2.7.8).
const USED_OFF_IDX: usize = 2; // §2.7.8
const USED_OFF_RING: usize = 4; // §2.7.8
const USED_ELEM_STRIDE: usize = 8; // §2.7.8

/// The rig's queue geometry and addresses — mirroring pci.rs's
/// bring_up_queue construction EXACTLY (qsize 8, RX, one shared header
/// page, rx_cap = 12 + one 4 KiB payload page).
const QSIZE: u16 = 8;
const HDR_PA: u64 = 0x1000_0000;
const PAY_PA: u64 = 0x2000_0000;
const PAYLOAD_CAP: u32 = 4096;
const RX_CAP: u32 = VIRTIO_NET_HDR_LEN as u32 + PAYLOAD_CAP;

/// slirp's ARP reply, byte-exact from the evidence capture
/// (.planning/phases/07-r9-pci-ecam-virtio-net-devether/evidence/07-07-virtnet-arp.pcap,
/// packet 2, incl_len 64): dst 52:54:00:12:34:56 (the guest MAC), src
/// 52:55:0a:00:02:02, etype 08 06 (2054), ARP reply oper 0002, spa
/// 10.0.2.2, tha the guest MAC, tpa 10.0.2.15, zero padding — the same
/// vector the round-2 diagnosis's scratch Arm A/Arm B proved against.
const PCAP_REPLY: [u8; 64] = [
    0x52, 0x54, 0x00, 0x12, 0x34, 0x56, 0x52, 0x55, 0x0a, 0x00, 0x02, 0x02, 0x08, 0x06, 0x00, 0x01,
    0x08, 0x00, 0x06, 0x04, 0x00, 0x02, 0x52, 0x55, 0x0a, 0x00, 0x02, 0x02, 0x0a, 0x00, 0x02, 0x02,
    0x52, 0x54, 0x00, 0x12, 0x34, 0x56, 0x0a, 0x00, 0x02, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// ---------------------------------------------------------------------------
// Mock ring memory (RingMem over a Vec arena — no op log: this module's
// assertions are byte-level; the barrier-order mock is virtio_tests'
// private affair)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct MockMem {
    buf: Rc<RefCell<Vec<u8>>>,
}

impl MockMem {
    fn new(size: usize) -> MockMem {
        MockMem { buf: Rc::new(RefCell::new(vec![0u8; size])) }
    }

    /// Device-side poke: direct buffer access, NOT through the driver's
    /// RingMem (the device does not log through the driver's interface).
    fn dev_write(&self, off: usize, bytes: &[u8]) {
        self.buf.borrow_mut()[off..off + bytes.len()].copy_from_slice(bytes);
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
        self.peek_u16(off)
    }
    fn read_u32(&self, off: usize) -> u32 {
        self.peek_u32(off)
    }
    fn read_u64(&self, off: usize) -> u64 {
        self.peek_u64(off)
    }
    fn write_u16(&self, off: usize, v: u16) {
        self.dev_write(off, &v.to_le_bytes());
    }
    fn write_u32(&self, off: usize, v: u32) {
        self.dev_write(off, &v.to_le_bytes());
    }
    fn write_u64(&self, off: usize, v: u64) {
        self.dev_write(off, &v.to_le_bytes());
    }
    // fence via the trait default (core's fence): no order log here.
}

// ---------------------------------------------------------------------------
// The rig + the device-side used-ring completion (virtio_tests' idiom)
// ---------------------------------------------------------------------------

struct Rig {
    ring: Ring<MockMem>,
    desc: MockMem,
    used: MockMem,
    notify: MockMem,
}

fn rig() -> Rig {
    let desc = MockMem::new(16 * QSIZE as usize);
    let avail = MockMem::new(6 + 2 * QSIZE as usize);
    let used = MockMem::new(6 + 8 * QSIZE as usize);
    let notify = MockMem::new(8);
    let ring = Ring::new(desc.clone(), avail, used.clone(), QSIZE, Dir::Rx, HDR_PA, RX_CAP)
        .expect("ring setup");
    Rig { ring, desc, used, notify }
}

/// Post slot 0 with its payload page at PAY_PA (one offer, then the
/// alloc dries up) — pci.rs's per-slot payload binding, and its ONE
/// notify for the burst.
fn replenish_slot0(rig: &mut Rig) {
    let mut once = Some((PAY_PA, PAYLOAD_CAP));
    let r = rig
        .ring
        .rx_replenish(|_slot| once.take(), &rig.notify, 0, virtio::VIRTIO_NET_QUEUE_RX);
    assert!(matches!(r, Ok(Replenish { offered: 1, notified: true })));
}

/// The device side of the used ring: write the entry at the DEVICE's own
/// used position (read back from the buffer — the device's cursor is
/// independent of the driver's last_used) and advance used.idx.  Direct
/// buffer pokes, per the virtio_tests device_complete idiom.
fn device_complete(rig: &Rig, id: u32, len: u32) {
    let dev_pos = rig.used.peek_u16(USED_OFF_IDX);
    let eoff = USED_OFF_RING + (dev_pos & (QSIZE - 1)) as usize * USED_ELEM_STRIDE;
    rig.used.dev_write(eoff, &id.to_le_bytes());
    rig.used.dev_write(eoff + 4, &len.to_le_bytes());
    rig.used.dev_write(USED_OFF_IDX, &dev_pos.wrapping_add(1).to_le_bytes());
}

/// The drain's slice arithmetic, mirrored from pci.rs's RX drain
/// EXACTLY: frame_len via the port helper, then index the payload-page
/// model at [virtio_net_frame_offset() .. offset + frame_len] — safe
/// indexing, so an offset defect panics loudly instead of reading
/// silently out of bounds.
fn drain_slice<'a>(rig: &mut Rig, payload_page: &'a [u8]) -> (UsedBuf, &'a [u8]) {
    let used = rig
        .ring
        .next_used()
        .expect("a used entry is pending")
        .expect("the entry passes next_used's validation");
    let frame_len = virtio::virtio_net_frame_len(used.len) as usize;
    let off = virtio::virtio_net_frame_offset();
    (used, &payload_page[off..off + frame_len])
}

// ---------------------------------------------------------------------------
// The pins
// ---------------------------------------------------------------------------

#[test]
fn rx_chain_layout_is_hdr_descriptor_then_payload_descriptor() {
    let mut rig = rig();
    replenish_slot0(&mut rig);
    // Descriptor 0 (slot 0's chain head): the 12-byte virtio_net_hdr
    // descriptor at the SHARED header page, NEXT-chained to the payload
    // descriptor (§5.1.6.3.2's chain shape — the descriptor boundary IS
    // the I/O-vector boundary the device writes across).
    let d0 = 0;
    assert_eq!(rig.desc.peek_u64(d0 + DESC_OFF_ADDR), HDR_PA);
    assert_eq!(rig.desc.peek_u32(d0 + DESC_OFF_LEN), VIRTIO_NET_HDR_LEN as u32);
    assert_eq!(rig.desc.peek_u16(d0 + DESC_OFF_FLAGS), DESC_F_NEXT | DESC_F_WRITE);
    assert_eq!(rig.desc.peek_u16(d0 + DESC_OFF_NEXT), 1);
    // Descriptor 1: the payload descriptor at PAY_PA, device-writable,
    // one full page.
    let d1 = DESC_STRIDE;
    assert_eq!(rig.desc.peek_u64(d1 + DESC_OFF_ADDR), PAY_PA);
    assert_eq!(rig.desc.peek_u32(d1 + DESC_OFF_LEN), PAYLOAD_CAP);
    assert_eq!(rig.desc.peek_u16(d1 + DESC_OFF_FLAGS), DESC_F_WRITE);
}

#[test]
fn rx_drain_slice_delivers_the_device_written_frame_byte_exact() {
    let mut rig = rig();
    replenish_slot0(&mut rig);
    // Device write per §5.1.6.3.2: the 12-byte virtio_net_hdr into the
    // SHARED header-page model (set dressing — with MRG_RXBUF off the
    // driver never consumes it; a nonzero sentinel makes the
    // separate-buffer truth visible), the frame into the payload page
    // AT OFFSET 0.
    let mut hdr_page = vec![0u8; 4096];
    hdr_page[..VIRTIO_NET_HDR_LEN].copy_from_slice(&[0x5au8; VIRTIO_NET_HDR_LEN]);
    let mut payload_page = vec![0u8; PAYLOAD_CAP as usize];
    payload_page[..PCAP_REPLY.len()].copy_from_slice(&PCAP_REPLY);
    // The used entry: slot 0's chain head (id 0), len = 12 + 64 = 76.
    device_complete(&rig, 0, (VIRTIO_NET_HDR_LEN + PCAP_REPLY.len()) as u32);
    let (used, slice) = drain_slice(&mut rig, &payload_page);
    assert_eq!(used, UsedBuf { slot: 0, len: 76 });
    assert_eq!(
        slice,
        &PCAP_REPLY[..],
        "the drain's slice must be the frame the device wrote, byte-exact"
    );
}

#[test]
fn rx_drain_slice_of_a_max_size_frame_stays_within_the_payload_page() {
    let mut rig = rig();
    replenish_slot0(&mut rig);
    // A max-size frame: used.len = 12 + 4096 = 4108 (the ring's
    // used_len_max — next_used's bound admits it).  The slice must cover
    // the WHOLE payload page and never read past it.
    let mut payload_page = vec![0u8; PAYLOAD_CAP as usize];
    payload_page[PAYLOAD_CAP as usize - 1] = 0xee; // the frame's last byte
    device_complete(&rig, 0, RX_CAP);
    let (used, slice) = drain_slice(&mut rig, &payload_page);
    assert_eq!(used.len, RX_CAP);
    assert_eq!(slice.len(), PAYLOAD_CAP as usize);
    assert_eq!(slice[PAYLOAD_CAP as usize - 1], 0xee, "the last byte is readable");
}
