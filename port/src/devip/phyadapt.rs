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
//!
//! The ether adapter (`EtherDev` over netif conversations 0x800/0x806/
//! 0x86dd) does NOT land here — it arrives with the ethermedium binding in
//! 08-06 (the Device impl shape is already proven by this module; adding
//! EtherDev then is expansion, not an architecture change).

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

/// The seam's concrete device sum (smoltcp's `Device` is not
/// dyn-compatible — GATs — so the mediums unify as an enum, dispatching
/// token types).  `EtherDev` joins in 08-06 with the ethermedium binding.
pub enum StackDev {
    Loopback(LoopbackDev),
    #[cfg(test)]
    Scripted(ScriptedDev),
}

impl StackDev {
    /// Packets queued for ingress (the drive loop's progress check —
    /// see drive_stack's loopback-feedback note).
    pub fn rx_pending(&self) -> bool {
        match self {
            StackDev::Loopback(dev) => dev.rx_pending(),
            #[cfg(test)]
            StackDev::Scripted(dev) => dev.rx_pending(),
        }
    }
}

/// The unified RX token (enum dispatch — same shape as `StackDev`).
pub enum StackRx {
    Loop(LoopRx),
    #[cfg(test)]
    Script(ScriptRx),
}

impl RxToken for StackRx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        match self {
            StackRx::Loop(rx) => rx.consume(f),
            #[cfg(test)]
            StackRx::Script(rx) => rx.consume(f),
        }
    }
}

/// The unified TX token (enum dispatch).
pub enum StackTx<'a> {
    Loop(LoopTx<'a>),
    #[cfg(test)]
    Script(ScriptTx<'a>),
}

impl TxToken for StackTx<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        match self {
            StackTx::Loop(tx) => tx.consume(len, f),
            #[cfg(test)]
            StackTx::Script(tx) => tx.consume(len, f),
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
            #[cfg(test)]
            StackDev::Scripted(dev) => {
                dev.receive(timestamp).map(|(rx, tx)| (StackRx::Script(rx), StackTx::Script(tx)))
            }
        }
    }

    fn transmit(&mut self, timestamp: Instant) -> Option<StackTx<'_>> {
        match self {
            StackDev::Loopback(dev) => dev.transmit(timestamp).map(StackTx::Loop),
            #[cfg(test)]
            StackDev::Scripted(dev) => dev.transmit(timestamp).map(StackTx::Script),
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        match self {
            StackDev::Loopback(dev) => dev.capabilities(),
            #[cfg(test)]
            StackDev::Scripted(dev) => dev.capabilities(),
        }
    }
}
