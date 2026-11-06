//! devip/phyadapt.rs — the smoltcp `Device` adapters (contract §12.1/§12.9).
//!
//! This plan lands the LOOPBACK adapter and the host-test capture device:
//!
//! - `LoopbackDev` — a bounded whole-packet loopback: `Medium::Ip` (no
//!   ethernet header on the loopback path), MTU 16384 (devip's loopback
//!   maxtu — contract §10, donor `loopbackmedium.c:11-13`), transmit pushes
//!   onto a bounded queue, receive pops.  NEVER smoltcp's `phy::Loopback`:
//!   its TxToken allocates per packet into an UNBOUNDED queue with no drop
//!   accounting (RESEARCH anti-pattern; the wrong posture for the kernel).
//!   Ours is bounded (`LOOPBACK_Q_CAP` packets ≈ the donor's 1 MiB Qmsg
//!   queue at maxtu) and a full queue drops + counts loudly.  Allocation
//!   happens ONLY in syscall context — the loopback is synchronous by
//!   construction (a TX write loops the packet in the same file-op context;
//!   no IRQ ever touches this device), so the per-packet Vec stays out of
//!   every IRQ path.
//! - `ScriptedDev` (cfg(test) only) — the scripted-RX / TX-capture device
//!   the host pins drive (the devether_tests stub discipline): tests push
//!   scripted inbound packets and assert on the captured TX bytes.
//! - `EtherDev` (08-06) — the ether adapter: `Medium::Ethernet` over
//!   Phase 7's devether (the ethermedium binding; netif conversations
//!   0x800/0x806/0x86dd merged in arrival order, TX through the registered
//!   tx_post, the /net/arp snoop on both directions).  `ScriptedEtherDev`
//!   (cfg(test)) is its host-pin mirror (whole-frame scripting with
//!   explicit arrival stamps; TX captured; the snoop live).
//!
//! The ether adapter's own section sits below the scripted loopback
//! device (the construction/merge/staging/snoop design notes are on it).

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;

/// devip's loopback maxtu (contract §10; `loopbackmedium.c:11-13`).
pub const LOOPBACK_MTU: usize = 16384;
/// The loopback queue bound in whole packets — the donor's 1 MiB Qmsg
/// queue (`loopbackmedium.c`) at maxtu granularity.  A full queue drops
/// and counts (never grows silently, never blocks the stack).
pub const LOOPBACK_Q_CAP: usize = 64;

/// The fixed loopback Device: a bounded VecDeque of whole packets.
/// transmit pushes, receive pops (the seam's synchronous drive replaces
/// the donor's loopback kproc reader — r9 has no kprocs by design).
pub struct LoopbackDev {
    queue: VecDeque<Vec<u8>>,
    /// Packets dropped on a full queue (the donor's qpass-failure arm —
    /// counted, never silent).
    dropped: u64,
}

impl LoopbackDev {
    pub fn new() -> LoopbackDev {
        LoopbackDev { queue: VecDeque::new(), dropped: 0 }
    }

    /// Packets queued for ingress (the seam's drive loop's progress
    /// check): a TX loops the packet INTO the RX queue — egress produces
    /// ingress work, so drive_stack loops until this drains.
    pub fn rx_pending(&self) -> bool {
        !self.queue.is_empty()
    }

    /// The drop counter (observability; the netlog wiring is 08-05's).
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

impl Default for LoopbackDev {
    fn default() -> LoopbackDev {
        LoopbackDev::new()
    }
}

/// A loopback RX token: one owned whole packet.
pub struct LoopRx {
    packet: Vec<u8>,
}

impl RxToken for LoopRx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.packet)
    }
}

/// A loopback TX token: loops the emitted packet back onto the queue.
pub struct LoopTx<'a> {
    queue: &'a mut VecDeque<Vec<u8>>,
    dropped: &'a mut u64,
}

impl TxToken for LoopTx<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let n = len.min(LOOPBACK_MTU);
        let mut buf = alloc::vec![0u8; n];
        let r = f(&mut buf);
        if self.queue.len() < LOOPBACK_Q_CAP {
            self.queue.push_back(buf);
        } else {
            *self.dropped += 1;
        }
        r
    }
}

impl Device for LoopbackDev {
    type RxToken<'a> = LoopRx;
    type TxToken<'a> = LoopTx<'a>;

    fn receive(&mut self, _timestamp: Instant) -> Option<(LoopRx, LoopTx<'_>)> {
        let packet = self.queue.pop_front()?;
        Some((LoopRx { packet }, LoopTx { queue: &mut self.queue, dropped: &mut self.dropped }))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<LoopTx<'_>> {
        Some(LoopTx { queue: &mut self.queue, dropped: &mut self.dropped })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = LOOPBACK_MTU;
        // checksum stays default (software) — the hardware computes nothing
        // (Phase 7's zero-offload mask; Pitfall 6 closed by construction).
        caps
    }
}

/// The scripted RX/TX capture Device the host pins drive (devether_tests
/// precedent).  TX frames are CAPTURED into `tx_log` (the pin asserts on
/// the wire bytes); scripted inbound packets are pushed into `rx` by the
/// test.  Allocation per packet is fine here — this device exists only in
/// host test builds.
#[cfg(test)]
pub struct ScriptedDev {
    pub rx: VecDeque<Vec<u8>>,
    pub tx_log: Vec<Vec<u8>>,
}

#[cfg(test)]
impl ScriptedDev {
    pub fn new() -> ScriptedDev {
        ScriptedDev { rx: VecDeque::new(), tx_log: Vec::new() }
    }

    /// Queue a scripted inbound packet (bare IPv4/IPv6 — Medium::Ip).
    pub fn push_rx(&mut self, packet: Vec<u8>) {
        self.rx.push_back(packet);
    }

    /// Scripted packets awaiting ingress (the seam's drive loop's
    /// progress check).
    pub fn rx_pending(&self) -> bool {
        !self.rx.is_empty()
    }
}

#[cfg(test)]
impl Default for ScriptedDev {
    fn default() -> ScriptedDev {
        ScriptedDev::new()
    }
}

/// A scripted RX token: one owned scripted packet.
#[cfg(test)]
pub struct ScriptRx {
    packet: Vec<u8>,
}

#[cfg(test)]
impl RxToken for ScriptRx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.packet)
    }
}

/// A scripted TX token: captures the emitted bytes into the log.
#[cfg(test)]
pub struct ScriptTx<'a> {
    log: &'a mut Vec<Vec<u8>>,
}

#[cfg(test)]
impl TxToken for ScriptTx<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = alloc::vec![0u8; len];
        let r = f(&mut buf);
        self.log.push(buf);
        r
    }
}

#[cfg(test)]
impl Device for ScriptedDev {
    type RxToken<'a> = ScriptRx;
    type TxToken<'a> = ScriptTx<'a>;

    fn receive(&mut self, _timestamp: Instant) -> Option<(ScriptRx, ScriptTx<'_>)> {
        let packet = self.rx.pop_front()?;
        Some((ScriptRx { packet }, ScriptTx { log: &mut self.tx_log }))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<ScriptTx<'_>> {
        Some(ScriptTx { log: &mut self.tx_log })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = LOOPBACK_MTU;
        caps
    }
}

// ---------------------------------------------------------------------------
// The ether adapter (08-06): `EtherDev` — smoltcp's `Medium::Ethernet`
// Device over Phase 7's devether (the ethermedium binding, contract §10/
// §12.9).
//
// CONSTRUCTION (`bind ether <path>`): opens the three netif
// conversations 0x800/0x806/0x86dd against the single published
// virtio-net device via the netif conv machinery (conv_alloc +
// conv_connect per type — NO file walk kernel-side; the donor's
// ethermedium.c chandial triple `path!0x800/0x806/0x86dd` mapped to
// direct calls, RESEARCH Pattern 3).  The negotiated MAC and the
// driver's tx_post come from devether's published EtherDriverOps.
//
// RECEIVE: merges the three conv queues in ARRIVAL order (the global
// ingress seq stamps, netif.rs) — smoltcp pulls one frame per
// poll_ingress_single.  TRANSMIT: posts whole frames through the
// registered EtherDriverOps tx_post (the 07-03 barrier-first ring
// machinery is unchanged).  A ring-FULL post can never fail smoltcp's
// consume (TxToken::consume is infallible), so the frame STAGES one deep
// and retries at the top of the next drive (the gate's traffic never
// fills the 256-slot ring; the retire-adjacent retry rides the next
// drive's timer/file-op/RX-wake leg).
//
// THE /net/arp SNOOP lives here (RESEARCH Pattern 4): BOTH receive() and
// transmit() feed arp.rs's snoop_frame (smoltcp's wire codecs parse the
// frames — never hand-rolled offsets); the TX side is the feedback leg
// (our own solicitations record WAIT) — the Phase-7 netif TX
// self-delivery gap is moot for smoltcp, but the snoop wants TX frames,
// so it lives at the ADAPTER level.
//
// capabilities(): Medium::Ethernet (which switches ON smoltcp's internal
// ARP/NDISC) + MTU 1514 — INCLUDES the 14-byte Ethernet header, excludes
// FCS (the convention trap; matches devether's ETHERMAXTU/mtu=1514
// exactly, NEVER 1500) + software checksums (Phase 7's zero-offload mask
// means the hardware computes nothing — Pitfall 6 closed by
// construction).
// ---------------------------------------------------------------------------

/// The ether MTU (devether's ETHERMAXTU): 1514 — INCLUDES the 14-byte
/// header (contract §10's convention trap).
pub const ETHER_MTU: usize = 1514;

/// The three ether-lane ethertypes (the ethermedium bind dial triple):
/// IPv4, ARP, IPv6.
pub(crate) const ETHER_LANES: [i32; 3] = [0x800, 0x806, 0x86dd];

/// Craft the gratuitous-ARP frame (the donor ethermedium's
/// areg=sendgarp — "whenever a new address is configured onto an
/// Ethernet, an ARP request is sent"; contract §10): a who-has <ip>
/// tell <ip> REQUEST (spa == tpa == the new address), broadcast, from
/// `mac` — via smoltcp's wire codecs (an allowed wire-codec use —
/// never a hand-rolled offset).  Shared by EtherDev (posts) and its
/// scripted mirror (captures); the pin parses the bytes it returns.
pub(crate) fn craft_gratuitous_arp(mac: [u8; 6], ip: [u8; 4]) -> Vec<u8> {
    use smoltcp::wire::{
        ArpOperation, ArpPacket, ArpRepr, EthernetAddress, EthernetFrame, EthernetProtocol,
        EthernetRepr, Ipv4Address,
    };
    let repr = ArpRepr::EthernetIpv4 {
        operation: ArpOperation::Request,
        source_hardware_addr: EthernetAddress(mac),
        source_protocol_addr: Ipv4Address::from(ip),
        target_hardware_addr: EthernetAddress([0; 6]),
        target_protocol_addr: Ipv4Address::from(ip),
    };
    let mut frame = alloc::vec![0u8; 14 + 28];
    let mut ef = EthernetFrame::new_unchecked(&mut frame[..]);
    EthernetRepr {
        src_addr: EthernetAddress(mac),
        dst_addr: EthernetAddress([0xff; 6]),
        ethertype: EthernetProtocol::Arp,
    }
    .emit(&mut ef);
    let mut pkt = ArpPacket::new_unchecked(ef.payload_mut());
    repr.emit(&mut pkt);
    frame
}

/// The ether Device over the netif conversations.
pub struct EtherDev {
    /// The three conversation ids ([IPv4, ARP, IPv6] lane order).
    convs: [usize; 3],
    /// The negotiated MAC (the Interface's hardware addr + the
    /// gratuitous-ARP sender).
    mac: [u8; 6],
    /// The driver's TX post (the registered EtherDriverOps op).
    tx_post: fn(&[u8]) -> crate::Result<bool>,
    /// One staged TX frame (the ring-full staging — see the header).
    staged: Option<Vec<u8>>,
    /// Frames dropped on a TX-post ERROR (counted, never silent).
    dropped: u64,
}

impl EtherDev {
    /// The ethermedium bind: allocate + connect the three conversations
    /// and capture the driver ops.  Errors roll the partial allocation
    /// back (a half-bound adapter never leaks convs).  `devether: no
    /// ether0` when the driver never published (the attach-era error
    /// shape, surfaced at bind).
    pub(crate) fn bind() -> crate::Result<EtherDev> {
        let ops = crate::devether::driver_ops()?;
        let mut convs = [0usize; 3];
        let mut n = 0;
        for (i, &etype) in ETHER_LANES.iter().enumerate() {
            match crate::netif::conv_alloc() {
                Ok(id) => {
                    if let Err(e) = crate::netif::conv_connect(id, etype) {
                        crate::netif::conv_close(id);
                        for &id in &convs[..n] {
                            crate::netif::conv_close(id);
                        }
                        return Err(e);
                    }
                    convs[i] = id;
                    n += 1;
                }
                Err(e) => {
                    for &id in &convs[..n] {
                        crate::netif::conv_close(id);
                    }
                    return Err(e);
                }
            }
        }
        Ok(EtherDev { convs, mac: ops.mac, tx_post: ops.tx_post, staged: None, dropped: 0 })
    }

    /// The negotiated MAC (the ether core's Config hardware addr).
    pub(crate) fn mac(&self) -> [u8; 6] {
        self.mac
    }
    /// Retry the staged TX frame (drive_stack's top-of-drive call).
    /// Restages on a still-full ring; an error drops + counts.
    pub(crate) fn flush_tx(&mut self) {
        let Some(frame) = self.staged.take() else { return };
        match (self.tx_post)(&frame) {
            Ok(true) => {}
            Ok(false) => self.staged = Some(frame),
            Err(_) => self.dropped += 1,
        }
    }

    /// Frames queued for ingress across the three lanes (the drive
    /// loop's progress check).
    pub(crate) fn rx_pending(&self) -> bool {
        self.convs.iter().any(|&id| crate::netif::head_seq(id).is_some())
    }

    /// Pop the next frame in ARRIVAL order across the three lanes (the
    /// global ingress stamps — the lane with the smallest head stamp
    /// wins).
    fn pop_merged(&mut self) -> Option<Vec<u8>> {
        let mut best: Option<(usize, u64)> = None;
        for (lane, &id) in self.convs.iter().enumerate() {
            if let Some(seq) = crate::netif::head_seq(id) {
                if best.map_or(true, |(_, s)| seq < s) {
                    best = Some((lane, seq));
                }
            }
        }
        let (lane, _) = best?;
        let mut buf = alloc::vec![0u8; ETHER_MTU];
        match crate::netif::pop_frame(self.convs[lane], &mut buf) {
            crate::netif::Pop::Frame(n) => {
                buf.truncate(n as usize);
                Some(buf)
            }
            // Empty: raced away (the head peek and the pop are separate
            // cell entries — impossible under the IRQ-masked-syscall
            // invariant, tolerated); Closed: the conv died under the
            // adapter (a bind-teardown defect — tolerated as an empty
            // read, the binding never closes the convs this era).
            _ => None,
        }
    }

    /// Post one frame through the driver (the consume/flush shared
    /// tail): snoop TX-side FIRST (the view records our own emissions —
    /// RESEARCH Pattern 3), then post; a full ring stages.
    fn emit(&mut self, frame: Vec<u8>) {
        super::arp::snoop_frame(&frame, true);
        match (self.tx_post)(&frame) {
            Ok(true) => {}
            Ok(false) => self.staged = Some(frame),
            Err(_) => self.dropped += 1,
        }
    }

    /// The gratuitous ARP (the donor ethermedium's `areg = sendgarp`;
    /// contract §10): smoltcp does NOT send one on update_ip_addrs, so
    /// devip crafts it (craft_gratuitous_arp) and emits it through the
    /// driver.  Called on every successful ether-ifc address add
    /// (ipifc.rs's ether arm — via StackDev::send_gratuitous_arp).
    pub(crate) fn send_gratuitous_arp(&mut self, ip: [u8; 4]) {
        let frame = craft_gratuitous_arp(self.mac, ip);
        self.emit(frame);
    }

    /// The drop counter (observability).
    #[allow(dead_code)]
    pub(crate) fn dropped(&self) -> u64 {
        self.dropped
    }
}

impl Drop for EtherDev {
    /// The unbind path: ipifc's unbind drops the ether seam core, which
    /// owns this adapter — closing the three netif conversations here
    /// releases the ether types so a LATER re-bind can allocate them
    /// again.  Without this close the convs leaked open and the re-bind
    /// failed `devether: ether-type already in use` (the "the binding
    /// never closes the convs this era" note in pop_merged — first
    /// exercised live by Harvey's 08-09 composed net smoke's
    /// static→dhcp unbind+rebind).  conv_close's last-close reset (typ
    /// cleared, queue drained, blocked readers wake to EOF) is netif's
    /// own discipline.  Runs OUTSIDE the stack cell — the caller
    /// (ipifc's unbind) takes the core out of the cell and drops it
    /// after the guard drops; the seam never holds STACK across netif's
    /// locks (the bind path's discipline, mirrored).
    fn drop(&mut self) {
        for &id in &self.convs {
            crate::netif::conv_close(id);
        }
    }
}

/// An ether RX token: one owned whole frame (14-byte header included).
pub struct EtherRx {
    frame: Vec<u8>,
}

impl RxToken for EtherRx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.frame)
    }
}

/// An ether TX token: fills a bounded buffer, snoops TX-side, posts
/// through the driver (a full ring stages — see the module header).
pub struct EtherTx<'a> {
    dev: &'a mut EtherDev,
}

impl TxToken for EtherTx<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let n = len.min(ETHER_MTU);
        let mut buf = alloc::vec![0u8; n];
        let r = f(&mut buf);
        self.dev.emit(buf);
        r
    }
}

impl Device for EtherDev {
    type RxToken<'a> = EtherRx;
    type TxToken<'a> = EtherTx<'a>;

    fn receive(&mut self, _timestamp: Instant) -> Option<(EtherRx, EtherTx<'_>)> {
        let frame = self.pop_merged()?;
        // The RX-side snoop (replies/announcements complete the view's
        // WAIT entries) — the frame then flows into smoltcp.
        super::arp::snoop_frame(&frame, false);
        Some((EtherRx { frame }, EtherTx { dev: self }))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<EtherTx<'_>> {
        if self.staged.is_some() {
            // One frame in flight — smoltcp re-attempts the transmit at
            // the next drive (None here leaves the packet queued).
            return None;
        }
        Some(EtherTx { dev: self })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = ETHER_MTU; // INCLUDES the 14-byte header
        caps
    }
}

/// The scripted ETHER device (the 08-06 pins): Medium::Ethernet frames
/// pushed/popped as WHOLE frames (the 14-byte header included), TX
/// captured, the arp snoop live (the pins assert the view's feed).  The
/// per-lane queues carry explicit seq stamps so the merge-order pin
/// controls arrival order exactly.
#[cfg(test)]
pub struct ScriptedEtherDev {
    /// The three lanes (0x800/0x806/0x86dd order) of (seq, frame).
    pub lanes: [VecDeque<(u64, Vec<u8>)>; 3],
    /// The captured TX frames.
    pub tx_log: Vec<Vec<u8>>,
    /// The scripted MAC.
    pub mac: [u8; 6],
}

#[cfg(test)]
impl ScriptedEtherDev {
    pub fn new(mac: [u8; 6]) -> ScriptedEtherDev {
        ScriptedEtherDev { lanes: [VecDeque::new(), VecDeque::new(), VecDeque::new()], tx_log: Vec::new(), mac }
    }

    /// Push a scripted inbound frame on the lane for its ethertype
    /// (non-lane ethertypes park on lane 0 — the pins only steer lanes).
    pub fn push_rx(&mut self, seq: u64, frame: Vec<u8>) {
        let lane = if frame.len() >= 14 {
            match u16::from_be_bytes([frame[12], frame[13]]) {
                0x806 => 1,
                0x86dd => 2,
                _ => 0,
            }
        } else {
            0
        };
        self.lanes[lane].push_back((seq, frame));
    }

    /// Frames queued for ingress (the drive loop's progress check).
    pub fn rx_pending(&self) -> bool {
        self.lanes.iter().any(|q| !q.is_empty())
    }

    /// The scripted gratuitous-ARP (the pin's capture of the ipifc add
    /// arm's emission): snoop TX-side (the production emit's shape),
    /// then capture into tx_log.
    pub fn send_gratuitous_arp(&mut self, ip: [u8; 4]) {
        let frame = craft_gratuitous_arp(self.mac, ip);
        super::arp::snoop_frame(&frame, true);
        self.tx_log.push(frame);
    }

    fn pop_merged(&mut self) -> Option<Vec<u8>> {
        let mut best: Option<(usize, u64)> = None;
        for (lane, q) in self.lanes.iter().enumerate() {
            if let Some(&(seq, _)) = q.front() {
                if best.map_or(true, |(_, s)| seq < s) {
                    best = Some((lane, seq));
                }
            }
        }
        let (lane, _) = best?;
        self.lanes[lane].pop_front().map(|(_, f)| f)
    }
}

/// The scripted ether RX token (whole frame).
#[cfg(test)]
pub struct ScriptEtherRx {
    frame: Vec<u8>,
}

#[cfg(test)]
impl RxToken for ScriptEtherRx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.frame)
    }
}

/// The scripted ether TX token: snoop TX-side, then capture.
#[cfg(test)]
pub struct ScriptEtherTx<'a> {
    dev: &'a mut ScriptedEtherDev,
}

#[cfg(test)]
impl TxToken for ScriptEtherTx<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = alloc::vec![0u8; len];
        let r = f(&mut buf);
        super::arp::snoop_frame(&buf, true);
        self.dev.tx_log.push(buf);
        r
    }
}

#[cfg(test)]
impl Device for ScriptedEtherDev {
    type RxToken<'a> = ScriptEtherRx;
    type TxToken<'a> = ScriptEtherTx<'a>;

    fn receive(&mut self, _timestamp: Instant) -> Option<(ScriptEtherRx, ScriptEtherTx<'_>)> {
        let frame = self.pop_merged()?;
        super::arp::snoop_frame(&frame, false);
        Some((ScriptEtherRx { frame }, ScriptEtherTx { dev: self }))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<ScriptEtherTx<'_>> {
        Some(ScriptEtherTx { dev: self })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = ETHER_MTU;
        caps
    }
}

/// The seam's concrete device sum (smoltcp's `Device` is not
/// dyn-compatible — GATs — so the mediums unify as an enum, dispatching
/// token types).  `EtherDev` joined in 08-06 with the ethermedium
/// binding; `ScriptedEtherDev` is its host-pin mirror.
pub enum StackDev {
    Loopback(LoopbackDev),
    Ether(EtherDev),
    #[cfg(test)]
    Scripted(ScriptedDev),
    #[cfg(test)]
    ScriptedEther(ScriptedEtherDev),
}

impl StackDev {
    /// Packets queued for ingress (the drive loop's progress check —
    /// see drive_stack's loopback-feedback note).
    pub fn rx_pending(&self) -> bool {
        match self {
            StackDev::Loopback(dev) => dev.rx_pending(),
            StackDev::Ether(dev) => dev.rx_pending(),
            #[cfg(test)]
            StackDev::Scripted(dev) => dev.rx_pending(),
            #[cfg(test)]
            StackDev::ScriptedEther(dev) => dev.rx_pending(),
        }
    }

    /// The ether adapter's staged-TX retry (a no-op for the other
    /// devices — the loopback never stages; the scripted devices capture
    /// unconditionally).
    pub fn flush_tx(&mut self) {
        if let StackDev::Ether(dev) = self {
            dev.flush_tx();
        }
    }

    /// The gratuitous-ARP emit (the ipifc ether arm's call): ether
    /// devices emit it (post/capture); non-ether devices have no garp
    /// (a no-op — the loopback medium's Medium struct has no areg).
    pub fn send_gratuitous_arp(&mut self, ip: [u8; 4]) {
        match self {
            StackDev::Ether(dev) => dev.send_gratuitous_arp(ip),
            #[cfg(test)]
            StackDev::ScriptedEther(dev) => dev.send_gratuitous_arp(ip),
            _ => {}
        }
    }

    /// The device's MAC (the ipifc `add6` verb's pref2addr source —
    /// ethermedium's EUI-64 derivation); None for non-ether devices (the
    /// loopback medium has no pref2addr — the donor's error arm).
    pub fn mac(&self) -> Option<[u8; 6]> {
        match self {
            StackDev::Ether(dev) => Some(dev.mac()),
            #[cfg(test)]
            StackDev::ScriptedEther(dev) => Some(dev.mac),
            _ => None,
        }
    }
}

/// The unified RX token (enum dispatch — same shape as `StackDev`).
pub enum StackRx {
    Loop(LoopRx),
    Ether(EtherRx),
    #[cfg(test)]
    Script(ScriptRx),
    #[cfg(test)]
    ScriptEther(ScriptEtherRx),
}

impl RxToken for StackRx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        match self {
            StackRx::Loop(rx) => rx.consume(f),
            StackRx::Ether(rx) => rx.consume(f),
            #[cfg(test)]
            StackRx::Script(rx) => rx.consume(f),
            #[cfg(test)]
            StackRx::ScriptEther(rx) => rx.consume(f),
        }
    }
}

/// The unified TX token (enum dispatch).
pub enum StackTx<'a> {
    Loop(LoopTx<'a>),
    Ether(EtherTx<'a>),
    #[cfg(test)]
    Script(ScriptTx<'a>),
    #[cfg(test)]
    ScriptEther(ScriptEtherTx<'a>),
}

impl TxToken for StackTx<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        match self {
            StackTx::Loop(tx) => tx.consume(len, f),
            StackTx::Ether(tx) => tx.consume(len, f),
            #[cfg(test)]
            StackTx::Script(tx) => tx.consume(len, f),
            #[cfg(test)]
            StackTx::ScriptEther(tx) => tx.consume(len, f),
        }
    }
}

impl Device for StackDev {
    type RxToken<'a> = StackRx;
    type TxToken<'a> = StackTx<'a>;

    fn receive(&mut self, timestamp: Instant) -> Option<(StackRx, StackTx<'_>)> {
        match self {
            StackDev::Loopback(dev) => {
                dev.receive(timestamp).map(|(rx, tx)| (StackRx::Loop(rx), StackTx::Loop(tx)))
            }
            StackDev::Ether(dev) => {
                dev.receive(timestamp).map(|(rx, tx)| (StackRx::Ether(rx), StackTx::Ether(tx)))
            }
            #[cfg(test)]
            StackDev::Scripted(dev) => {
                dev.receive(timestamp).map(|(rx, tx)| (StackRx::Script(rx), StackTx::Script(tx)))
            }
            #[cfg(test)]
            StackDev::ScriptedEther(dev) => {
                dev.receive(timestamp).map(|(rx, tx)| (StackRx::ScriptEther(rx), StackTx::ScriptEther(tx)))
            }
        }
    }

    fn transmit(&mut self, timestamp: Instant) -> Option<StackTx<'_>> {
        match self {
            StackDev::Loopback(dev) => dev.transmit(timestamp).map(StackTx::Loop),
            StackDev::Ether(dev) => dev.transmit(timestamp).map(StackTx::Ether),
            #[cfg(test)]
            StackDev::Scripted(dev) => dev.transmit(timestamp).map(StackTx::Script),
            #[cfg(test)]
            StackDev::ScriptedEther(dev) => dev.transmit(timestamp).map(StackTx::ScriptEther),
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        match self {
            StackDev::Loopback(dev) => dev.capabilities(),
            StackDev::Ether(dev) => dev.capabilities(),
            #[cfg(test)]
            StackDev::Scripted(dev) => dev.capabilities(),
            #[cfg(test)]
            StackDev::ScriptedEther(dev) => dev.capabilities(),
        }
    }
}
