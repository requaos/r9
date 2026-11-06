//! The devip seam host pins: homed at the CRATE root beside
//! allocator_tests.rs/virtio_tests.rs/devether_tests.rs/rx_chain_tests.rs
//! (the crate-root placement precedent — lib.rs's registration block; the
//! scripted-device drives and the stub wake log need alloc/std while
//! port::devip keeps its unsafe-free forbid attribute and the waker bridge
//! keeps its lone documented exception).
//!
//! The seam under pin is D-05's event-driven machinery: `drive_stack`, the
//! waker bridge (one-shot, re-register after EVERY wake, spurious-allowed),
//! the `poll_at`-armed one-shot deadline (`NET_DEADLINE`, tick units), the
//! accept-pool demux over smoltcp's no-backlog listen, and the stack-debt
//! drain.  smoltcp compiles for the host (probe-proven), so the entire
//! devip↔smoltcp mapping is pinned here BEFORE any guest boot gate depends
//! on it (the CONTEXT watch-out: the seam has no polling cadence to save it
//! from a missed waker).
//!
//! THE PIN MAP (RESEARCH §Pattern 1's pin list):
//!
//! - (a) `waker_bridge_fires_on_rx_close_and_buffer_space`
//! - (b) `register_wake_reregister_never_loses_a_wake`
//! - (c) `deadline_arms_from_poll_at_across_retransmit`
//! - (d) `accept_pool_demuxes_two_syns_to_two_members`
//! - (g) `stack_debt_drains_on_next_op`
//! - (h) `dhcp_shaped_exchange_sources_0_0_0_0_on_the_wire` — OQ-1, the
//!   phase's FIRST pin: the verdict (PASS-path vs SHIM-path) is recorded in
//!   docs/devip-contract.md §12.6.
//!
//! Test-harness discipline (devether_tests/rx_chain_tests precedents):
//!
//! - STUB HOOKS over test-module statics, registered ONCE per process via a
//!   std::sync::Once (the seam's set_hooks/set_clock assert-once, and every
//!   unit test in the crate shares one test binary).
//! - The stub clock is ADVANCEABLE (`TEST_TICKS`) — the deadline pins move
//!   time explicitly instead of sleeping (deterministic; 1 tick = 1 ms).
//! - The stateful pins SERIALIZE on `SEAM_TEST_LOCK` (the rust test harness
//!   is multi-threaded; the seam's statics are process-global) and each
//!   installs a FRESH scripted stack (`install_test_stack` resets the
//!   deadline/debt/wake sets).
//! - Wire packets are crafted/parsed with smoltcp's own `wire` codecs (the
//!   RESEARCH's allowed reuse — the wire layer is public and documented).
//!   Sequence numbers are NEVER hardcoded: the pins parse the seam's own
//!   captured SYN to learn the local ISN.

use std::sync::Mutex;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use smoltcp::iface::SocketHandle;
use smoltcp::phy::ChecksumCapabilities;
use smoltcp::socket::tcp;
use smoltcp::socket::udp;
use smoltcp::socket::udp::UdpMetadata;
use smoltcp::storage::PacketBuffer;
use smoltcp::storage::PacketMetadata;
use smoltcp::wire::IpAddress;
use smoltcp::wire::IpCidr;
use smoltcp::wire::IpEndpoint;
use smoltcp::wire::IpListenEndpoint;
use smoltcp::wire::IpProtocol;
use smoltcp::wire::Ipv4Address;
use smoltcp::wire::Ipv4Cidr;
use smoltcp::wire::Ipv4Packet;
use smoltcp::wire::Ipv4Repr;
use smoltcp::wire::TcpControl;
use smoltcp::wire::TcpPacket;
use smoltcp::wire::TcpRepr;
use smoltcp::wire::TcpSeqNumber;
use smoltcp::wire::UdpPacket;
use smoltcp::wire::UdpRepr;

use crate::devip::phyadapt::ScriptedDev;
use crate::devip::stack::{self, StackCore};
use crate::devip_waker::{self, WakeSlot};

// ---------------------------------------------------------------------------
// The stub hooks (the devpipe/devether shape: {current, block, wake}) and
// the advanceable stub clock.
// ---------------------------------------------------------------------------

/// The fixed pid the pins register (WaitSet.register silently drops pid 0).
const STUB_PID: u64 = 42;
/// A second pid for the direction-distinguishing waker pin.
const STUB_PID_TX: u64 = 43;

/// The wake hook's record: every pid the seam woke, in wake order.
static WAKE_LOG: Mutex<Vec<u64>> = Mutex::new(Vec::new());
/// The block hook's invocation count (08-05: the threaded tcp pins prove
/// the blocking paths PARKED — the connect/listen waits record their
/// blocks here before the scripted peer answers).
static BLOCK_COUNT: AtomicU64 = AtomicU64::new(0);
/// The advanceable stub clock (1 tick = 1 ms).
static TEST_TICKS: AtomicU64 = AtomicU64::new(0);
/// Serializes the stateful pins (process-global seam statics).
static SEAM_TEST_LOCK: Mutex<()> = Mutex::new(());
static HOOKS_ONCE: Once = Once::new();

/// The wakee slots the pins register from (statics — the waker bridge's
/// never-outlives-the-conv invariant made mechanical).
static SLOT_RX: WakeSlot = WakeSlot::new();
static SLOT_TX: WakeSlot = WakeSlot::new();

fn stub_current() -> u64 {
    STUB_PID
}

fn stub_block() {
    // Record the park (the blocking-path pins assert it).  The 08-05 tcp
    // pins run the scripted peer's answer from INSIDE the park: the
    // one-shot ON_BLOCK action fires first (the SYN-ACK/RST/hangup the
    // path is waiting for), then the standing BLOCK_RELAY loops new
    // captures back as RX (the full-exchange pin's handshake + data legs
    // complete inside the real blocking paths).  Both run with no seam
    // locks held (the block paths drop everything before blocking).
    BLOCK_COUNT.fetch_add(1, Ordering::Relaxed);
    if let Some(act) = ON_BLOCK.lock().unwrap_or_else(|e| e.into_inner()).take() {
        act();
    }
    if BLOCK_RELAY.load(Ordering::Relaxed) {
        relay_new_tx();
    }
    std::thread::yield_now();
}

fn stub_wake(pids: &[u64]) {
    WAKE_LOG.lock().unwrap().extend_from_slice(pids);
}

fn stub_ticks() -> u64 {
    TEST_TICKS.load(Ordering::Relaxed)
}

fn set_ticks(t: u64) {
    TEST_TICKS.store(t, Ordering::Relaxed);
}

fn take_wake_log() -> Vec<u64> {
    std::mem::take(&mut *WAKE_LOG.lock().unwrap_or_else(|e| e.into_inner()))
}

/// The block hook's invocation count (see BLOCK_COUNT).
fn block_count() -> u64 {
    BLOCK_COUNT.load(Ordering::Relaxed)
}

/// Bounded spin-wait for a host-side condition (the threaded tcp pins):
/// the condition must become true within ~5 s of yields or the pin FAILS
/// loudly — a wedge is a test failure, never a hung suite.
fn wait_until(mut cond: impl FnMut() -> bool, what: &str) {
    let start = std::time::Instant::now();
    while !cond() {
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "pin: wedged waiting for {what}"
        );
        std::thread::yield_now();
    }
}

/// Register the stub hooks + clock exactly once per process.
fn ensure_hooks() {
    HOOKS_ONCE.call_once(|| {
        stack::set_clock(stub_ticks);
        stack::set_hooks(stack::DevipHooks {
            current: stub_current,
            block: stub_block,
            wake: stub_wake,
        });
    });
}

/// Install a FRESH scripted stack with the clock at 0 and a clean wake log.
/// Returns the guard serializing this pin against the others.  Poison-tolerant:
/// a panicking pin must not cascade PoisonError into its siblings — the fresh
/// install below resets the whole seam state anyway.
fn fresh_scripted_stack() -> std::sync::MutexGuard<'static, ()> {
    let guard = SEAM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_hooks();
    set_ticks(0);
    take_wake_log();
    RELAYED.store(0, Ordering::Relaxed);
    BLOCK_RELAY.store(false, Ordering::Relaxed);
    stack::install_test_stack(StackCore::new_scripted(ScriptedDev::new()));
    guard
}

// ---------------------------------------------------------------------------
// Wire crafting/parsing (smoltcp's own codecs — never hand-rolled bytes).
// ScriptedDev is Medium::Ip: frames are bare IPv4 packets.
// ---------------------------------------------------------------------------

/// The scripted guest address (the iface under test) and peer.
const GUEST: Ipv4Address = Ipv4Address::new(10, 0, 0, 1);
const PEER: Ipv4Address = Ipv4Address::new(10, 0, 0, 2);
/// The scripted second peer (the accept-pool demux's second SYN).
const PEER2: Ipv4Address = Ipv4Address::new(10, 0, 0, 3);
/// The scripted DHCP server (the slirp shape).
const DHCP_SERVER: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);

/// A parsed TCP leg of a captured/scripted frame (the fields the pins
/// assert on).
struct TcpLeg {
    src: Ipv4Address,
    dst: Ipv4Address,
    sport: u16,
    dport: u16,
    seq: i32,
    ack: Option<i32>,
    syn: bool,
    fin: bool,
    rst: bool,
    payload: Vec<u8>,
}

/// Craft a bare-IPv4 UDP datagram with valid checksums.
fn udp4_frame(src: Ipv4Address, dst: Ipv4Address, sport: u16, dport: u16, payload: &[u8]) -> Vec<u8> {
    let mut bytes = alloc_vec(20 + 8 + payload.len());
    {
        let mut ip = Ipv4Packet::new_unchecked(&mut bytes);
        Ipv4Repr {
            src_addr: src,
            dst_addr: dst,
            next_header: IpProtocol::Udp,
            payload_len: 8 + payload.len(),
            hop_limit: 64,
        }
        .emit(&mut ip, &ChecksumCapabilities::default());
    }
    let ip_payload_range = 20..20 + 8 + payload.len();
    let mut udp = UdpPacket::new_unchecked(&mut bytes[ip_payload_range]);
    UdpRepr { src_port: sport, dst_port: dport }.emit(
        &mut udp,
        &IpAddress::Ipv4(src),
        &IpAddress::Ipv4(dst),
        payload.len(),
        |b| b.copy_from_slice(payload),
        &ChecksumCapabilities::default(),
    );
    bytes
}

/// Craft a bare-IPv4 TCP segment with valid checksums (no options — the
/// pins' scripted peer speaks minimal TCP; smoltcp tolerates missing MSS/TS
/// on the SYN-ACK, the same as its own test suite's segments).
fn tcp4_frame(
    src: Ipv4Address,
    dst: Ipv4Address,
    sport: u16,
    dport: u16,
    seq: i32,
    ack: Option<i32>,
    control: TcpControl,
    payload: &[u8],
) -> Vec<u8> {
    let repr = TcpRepr {
        src_port: sport,
        dst_port: dport,
        control,
        seq_number: TcpSeqNumber(seq),
        ack_number: ack.map(TcpSeqNumber),
        window_len: 65535,
        window_scale: None,
        max_seg_size: None,
        sack_permitted: false,
        sack_ranges: [None, None, None],
        timestamp: None,
        payload,
    };
    let mut bytes = alloc_vec(20 + repr.buffer_len());
    {
        let mut ip = Ipv4Packet::new_unchecked(&mut bytes);
        Ipv4Repr {
            src_addr: src,
            dst_addr: dst,
            next_header: IpProtocol::Tcp,
            payload_len: repr.buffer_len(),
            hop_limit: 64,
        }
        .emit(&mut ip, &ChecksumCapabilities::default());
    }
    let total = 20 + repr.buffer_len();
    let mut tcp = TcpPacket::new_unchecked(&mut bytes[20..total]);
    repr.emit(
        &mut tcp,
        &IpAddress::Ipv4(src),
        &IpAddress::Ipv4(dst),
        &ChecksumCapabilities::default(),
    );
    bytes
}

/// Parse the TCP leg of a captured bare-IPv4 frame (panics loudly on a
/// non-TCP or malformed frame — a pin's parse failure IS the failure).
fn parse_tcp4(frame: &[u8]) -> TcpLeg {
    let ip = Ipv4Packet::new_checked(frame).expect("pin: ip parse");
    let tcp = TcpPacket::new_checked(ip.payload()).expect("pin: tcp parse");
    TcpLeg {
        src: ip.src_addr(),
        dst: ip.dst_addr(),
        sport: tcp.src_port(),
        dport: tcp.dst_port(),
        seq: tcp.seq_number().0,
        ack: if tcp.ack() { Some(tcp.ack_number().0) } else { None },
        syn: tcp.syn(),
        fin: tcp.fin(),
        rst: tcp.rst(),
        payload: tcp.payload().to_vec(),
    }
}

/// Parse the UDP leg of a captured bare-IPv4 frame.
fn parse_udp4(frame: &[u8]) -> (Ipv4Address, Ipv4Address, u16, u16, Vec<u8>) {
    let ip = Ipv4Packet::new_checked(frame).expect("pin: ip parse");
    let udp = UdpPacket::new_checked(ip.payload()).expect("pin: udp parse");
    (
        ip.src_addr(),
        ip.dst_addr(),
        udp.src_port(),
        udp.dst_port(),
        udp.payload().to_vec(),
    )
}

fn alloc_vec(n: usize) -> Vec<u8> {
    std::vec![0u8; n]
}

// ---------------------------------------------------------------------------
// Shared seam surgery helpers (each enters the real STACK cell, does its
// work, and drops — drives happen only with the guard dropped, because the
// REAL drive_stack enters the cell itself).
// ---------------------------------------------------------------------------

/// Assign the guest address to the scripted interface.
fn set_guest_addr() {
    let mut g = stack::enter_stack();
    let core = g.core.as_mut().expect("pin: stack installed");
    core.iface.update_ip_addrs(|addrs| {
        addrs.push(IpCidr::Ipv4(Ipv4Cidr::new(GUEST, 24))).expect("pin: addr fits");
    });
}

/// Add a TCP socket (2048/2048 buffers, hop limit 255 per contract §12.8)
/// and return its handle.
fn add_tcp_socket() -> SocketHandle {
    let mut g = stack::enter_stack();
    let core = g.core.as_mut().expect("pin: stack installed");
    let mut s = tcp::Socket::new(
        tcp::SocketBuffer::new(std::vec![0; 2048]),
        tcp::SocketBuffer::new(std::vec![0; 2048]),
    );
    s.set_hop_limit(Some(255));
    core.sockets.add(s)
}

/// The captured TX frame count.
fn tx_count() -> usize {
    let g = stack::enter_stack();
    let core = g.core.as_ref().expect("pin: stack installed");
    core.dev_scripted().tx_log.len()
}

/// Parse the n-th captured TX frame as TCP.
fn tx_tcp(n: usize) -> TcpLeg {
    let g = stack::enter_stack();
    let core = g.core.as_ref().expect("pin: stack installed");
    let frame = &core.dev_scripted().tx_log[n];
    parse_tcp4(frame)
}

/// Push a scripted inbound frame.
fn push_rx(frame: Vec<u8>) {
    let mut g = stack::enter_stack();
    let core = g.core.as_mut().expect("pin: stack installed");
    core.dev_scripted_mut().push_rx(frame);
}

/// Register the RX waker on a TCP socket (the conv's arm-before-register
/// discipline: the slot is armed BEFORE the waker is registered).
fn register_rx_tcp(handle: SocketHandle) {
    let mut g = stack::enter_stack();
    let core = g.core.as_mut().expect("pin: stack installed");
    SLOT_RX.arm(STUB_PID);
    core.sockets
        .get_mut::<tcp::Socket>(handle)
        .register_recv_waker(&devip_waker::waker_for(&SLOT_RX));
}

/// Register the RX waker on a UDP socket.
fn register_rx_udp(handle: SocketHandle) {
    let mut g = stack::enter_stack();
    let core = g.core.as_mut().expect("pin: stack installed");
    SLOT_RX.arm(STUB_PID);
    core.sockets
        .get_mut::<udp::Socket>(handle)
        .register_recv_waker(&devip_waker::waker_for(&SLOT_RX));
}

/// Register the TX waker on a TCP socket.
fn register_tx_tcp(handle: SocketHandle) {
    let mut g = stack::enter_stack();
    let core = g.core.as_mut().expect("pin: stack installed");
    SLOT_TX.arm(STUB_PID_TX);
    core.sockets
        .get_mut::<tcp::Socket>(handle)
        .register_send_waker(&devip_waker::waker_for(&SLOT_TX));
}

/// Establish a scripted TCP connection: connect → SYN captured → scripted
/// SYN-ACK → ACK captured → Established.  Returns (handle, local_isn,
/// remote_isn).  The local ISN is PARSED from the seam's own SYN (never
/// hardcoded).
fn establish_tcp(local_port: u16, remote_port: u16) -> (SocketHandle, i32, i32) {
    const REMOTE_ISN: i32 = 1000;
    let handle = add_tcp_socket();
    {
        let mut g = stack::enter_stack();
        let core = g.core.as_mut().expect("pin: stack installed");
        let s = core.sockets.get_mut::<tcp::Socket>(handle);
        s.connect(core.iface.context(), (IpAddress::Ipv4(PEER), remote_port), local_port)
            .expect("pin: connect");
    }
    stack::drive_stack(); // SYN out
    let syn = tx_tcp(tx_count() - 1);
    assert!(syn.syn && syn.ack.is_none(), "pin: first frame is a bare SYN");
    assert_eq!(syn.src, GUEST);
    assert_eq!(syn.dst, PEER);
    assert_eq!(syn.sport, local_port);
    assert_eq!(syn.dport, remote_port);
    let local_isn = syn.seq;

    push_rx(tcp4_frame(
        PEER,
        GUEST,
        remote_port,
        local_port,
        REMOTE_ISN,
        Some(local_isn + 1),
        TcpControl::Syn,
        &[],
    ));
    stack::drive_stack(); // SYN-ACK in → ACK out
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin: stack installed");
        let s = core.sockets.get::<tcp::Socket>(handle);
        assert_eq!(s.state(), tcp::State::Established, "pin: established after handshake");
    }
    (handle, local_isn, REMOTE_ISN)
}

// ---------------------------------------------------------------------------
// Smoke: the seam constructs, installs, and drives clean (08-02 Task 1).
// ---------------------------------------------------------------------------

#[test]
fn seam_installs_and_drives_clean() {
    let _serial = fresh_scripted_stack();

    // An idle stack drives to quiescence: no ingress, no egress, no pending
    // timers → the deadline disarms and nothing wakes.
    stack::drive_stack();
    assert_eq!(stack::net_deadline(), None);
    assert_eq!(take_wake_log(), Vec::<u64>::new());

    // No debt, and a drain on a clear flag is a no-op.
    assert!(!stack::drain_debt_if_set());

    // A deadline that has not passed wakes nobody and sets no debt; one that
    // HAS passed with zero registered waiters sets the debt, and the drain
    // clears it and drives (idles) the stack.
    stack::NET_DEADLINE.store(1000, Ordering::Release);
    stack::tick_check(999);
    assert_eq!(take_wake_log(), Vec::<u64>::new());
    stack::tick_check(1000);
    assert!(stack::STACK_DEBT.load(Ordering::Acquire));
    assert!(stack::drain_debt_if_set());
    assert_eq!(stack::net_deadline(), None);
}

// ---------------------------------------------------------------------------
// Pin (h) — OQ-1 FIRST: ipconfig's exact DHCP-shaped exchange against a
// host-side smoltcp instance with NO configured interface address.
// The verdict lands in docs/devip-contract.md §12.6.
// ---------------------------------------------------------------------------

/// ipconfig's DHCP exchange shape (main.c:923-963): announce udp!*!68 →
/// headers mode → full-size Bootp datagrams, broadcast dst
/// 255.255.255.255:67 for Discover/Request.  The pin's payload stands in
/// for Bootp: the 236-byte fixed part is zeros here, the magic cookie is
/// real (99 130 83 99), and a marker tail proves payload integrity.
const DHCP_MARKER: &[u8] = b"DEVIP-OQ1-DISCOVER";
const OFFER_MARKER: &[u8] = b"DEVIP-OQ1-OFFER";

fn bootp_shaped_payload(marker: &[u8]) -> Vec<u8> {
    let mut p = alloc_vec(240); // the Bootp fixed part + magic cookie
    p[236..240].copy_from_slice(&[99, 130, 83, 99]);
    p.extend_from_slice(marker);
    p
}

#[test]
fn dhcp_shaped_exchange_sources_0_0_0_0_on_the_wire() {
    let _serial = fresh_scripted_stack();

    // NO configured interface address — the DHCP preconfig state.  (The
    // `add 0.0.0.0 0.0.0.0` write is absorbed devip-side per contract
    // §12.6 — smoltcp's update_ip_addrs would PANIC on it.)
    let handle = {
        let mut g = stack::enter_stack();
        let core = g.core.as_mut().expect("pin: stack installed");
        let mut s = udp::Socket::new(
            PacketBuffer::new(std::vec![PacketMetadata::EMPTY; 4], alloc_vec(4096)),
            PacketBuffer::new(std::vec![PacketMetadata::EMPTY; 4], alloc_vec(4096)),
        );
        // ipconfig: announce("udp!*!68") — port 68, no address.
        s.bind(IpListenEndpoint { addr: None, port: 68 }).expect("pin: bind 68");
        s.set_hop_limit(Some(255)); // the bridge's discipline (contract §12.8)
        core.sockets.add(s)
    };

    // LEG 1 — the NAIVE shape (no explicit local_address): the datagram
    // must NOT leave.  smoltcp's udp dispatch drops it SILENTLY
    // (get_source_address returns None on an address-less interface;
    // net_trace only, no error) — this leg pins WHY the bridge must set
    // UdpMetadata.local_address explicitly from the conv's recorded laddr.
    {
        let mut g = stack::enter_stack();
        let core = g.core.as_mut().expect("pin: stack installed");
        let s = core.sockets.get_mut::<udp::Socket>(handle);
        s.send_slice(
            &bootp_shaped_payload(DHCP_MARKER),
            IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::BROADCAST), 67),
        )
        .expect("pin: naive send enqueues (the drop is at dispatch)");
    }
    stack::drive_stack();
    assert_eq!(tx_count(), 0, "pin: the naive shape is silently dropped (no src address)");

    // LEG 2 — the BRIDGE shape: local_address = Some(0.0.0.0) (the conv's
    // absorbed-DHCP laddr).  The Discover MUST leave with src 0.0.0.0,
    // dst 255.255.255.255, udp 68 → 67, payload intact.
    {
        let mut g = stack::enter_stack();
        let core = g.core.as_mut().expect("pin: stack installed");
        let s = core.sockets.get_mut::<udp::Socket>(handle);
        let mut meta = UdpMetadata::from(IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::BROADCAST), 67));
        meta.local_address = Some(IpAddress::Ipv4(Ipv4Address::UNSPECIFIED));
        s.send_slice(&bootp_shaped_payload(DHCP_MARKER), meta).expect("pin: bridge send");
    }
    stack::drive_stack();
    assert_eq!(tx_count(), 1, "pin: the bridge shape emits exactly one frame");
    let (src, dst, sport, dport, payload) = {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin: stack installed");
        parse_udp4(&core.dev_scripted().tx_log[0])
    };
    assert_eq!(src, Ipv4Address::UNSPECIFIED, "OQ-1 verdict: src 0.0.0.0 ON THE WIRE");
    assert_eq!(dst, Ipv4Address::BROADCAST);
    assert_eq!((sport, dport), (68, 67));
    assert!(payload.ends_with(DHCP_MARKER), "pin: payload integrity");

    // LEG 3 — the OFFER comes back: broadcast dst to our port 68 with STILL
    // no configured address (the slirp DHCP server's shape).  The socket
    // must accept it (udp accepts(): port match; broadcast dst).
    push_rx(udp4_frame(DHCP_SERVER, Ipv4Address::BROADCAST, 67, 68, &bootp_shaped_payload(OFFER_MARKER)));
    stack::drive_stack();
    {
        let mut g = stack::enter_stack();
        let core = g.core.as_mut().expect("pin: stack installed");
        let s = core.sockets.get_mut::<udp::Socket>(handle);
        assert!(s.can_recv(), "pin: the broadcast Offer is accepted with no iface address");
        let mut buf = alloc_vec(4096);
        let (n, meta) = s.recv_slice(&mut buf).expect("pin: recv the Offer");
        assert_eq!(meta.endpoint.addr, IpAddress::Ipv4(DHCP_SERVER));
        assert_eq!(meta.endpoint.port, 67);
        assert!(buf[..n].ends_with(OFFER_MARKER), "pin: Offer payload integrity");
    }
}

// ---------------------------------------------------------------------------
// Pin (a) — the waker bridge fires on socket RX data, on peer close, and on
// send-buffer space; the recorded wakee pid is woken exactly once per
// registration.
// ---------------------------------------------------------------------------

#[test]
fn waker_bridge_fires_on_rx_close_and_buffer_space() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();

    let (handle, local_isn, remote_isn) = establish_tcp(49152, 1234);

    // The handshake's state transitions fire registered wakers; we register
    // AFTER establish, so the log starts empty here.
    assert_eq!(take_wake_log(), Vec::<u64>::new());

    // EVENT 1 — RX data: register the recv waker, inject a data segment.
    register_rx_tcp(handle);
    push_rx(tcp4_frame(
        PEER,
        GUEST,
        1234,
        49152,
        remote_isn + 1,
        Some(local_isn + 1),
        TcpControl::Psh,
        b"hello",
    ));
    stack::drive_stack();
    assert_eq!(take_wake_log(), std::vec![STUB_PID], "pin (a): RX data woke the wakee once");
    {
        let mut g = stack::enter_stack();
        let core = g.core.as_mut().expect("pin: stack installed");
        let s = core.sockets.get_mut::<tcp::Socket>(handle);
        assert!(s.can_recv());
        let mut buf = alloc_vec(64);
        let n = s.recv_slice(&mut buf).expect("pin: recv");
        assert_eq!(&buf[..n], b"hello");
    }

    // EVENT 2 — send-buffer space: fill a TX slice, drive it out, then the
    // peer's ACK frees the buffer → the SEND waker fires.
    register_tx_tcp(handle);
    {
        let mut g = stack::enter_stack();
        let core = g.core.as_mut().expect("pin: stack installed");
        let s = core.sockets.get_mut::<tcp::Socket>(handle);
        s.send_slice(b"xyz").expect("pin: send");
    }
    stack::drive_stack(); // data out
    assert_eq!(take_wake_log(), Vec::<u64>::new(), "pin (a): no wake before the ACK");
    push_rx(tcp4_frame(
        PEER,
        GUEST,
        1234,
        49152,
        remote_isn + 6,
        Some(local_isn + 4), // acking "xyz"
        TcpControl::None,
        &[],
    ));
    stack::drive_stack();
    assert_eq!(take_wake_log(), std::vec![STUB_PID_TX], "pin (a): freed buffer woke the TX wakee once");

    // EVENT 3 — peer close: FIN → CloseWait → the recv waker fires (a
    // pending read must observe the close).
    register_rx_tcp(handle);
    push_rx(tcp4_frame(
        PEER,
        GUEST,
        1234,
        49152,
        remote_isn + 6,
        Some(local_isn + 4),
        TcpControl::Fin,
        &[],
    ));
    stack::drive_stack();
    assert_eq!(take_wake_log(), std::vec![STUB_PID], "pin (a): peer close woke the wakee once");
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin: stack installed");
        let s = core.sockets.get::<tcp::Socket>(handle);
        assert_eq!(s.state(), tcp::State::CloseWait);
    }
}

// ---------------------------------------------------------------------------
// Pin (b) — the lost-wakeup shape: register → scripted wake → re-register
// cycles never lose a wake; a quiet drive never consumes the waker.
// ---------------------------------------------------------------------------

#[test]
fn register_wake_reregister_never_loses_a_wake() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();

    let handle = {
        let mut g = stack::enter_stack();
        let core = g.core.as_mut().expect("pin: stack installed");
        let mut s = udp::Socket::new(
            PacketBuffer::new(std::vec![PacketMetadata::EMPTY; 4], alloc_vec(4096)),
            PacketBuffer::new(std::vec![PacketMetadata::EMPTY; 4], alloc_vec(4096)),
        );
        s.bind(68).expect("pin: bind");
        core.sockets.add(s)
    };

    for cycle in 0..3 {
        // The conv's discipline: arm the slot BEFORE registering; every wake
        // is followed by a re-registration and a can_recv re-check
        // (spurious-wake discipline — a wake guarantees nothing).
        register_rx_udp(handle);
        push_rx(udp4_frame(PEER, GUEST, 1000 + cycle, 68, &[cycle as u8]));
        stack::drive_stack();
        assert_eq!(
            take_wake_log(),
            std::vec![STUB_PID],
            "pin (b): cycle {cycle} — the wake fired exactly once"
        );

        // The waker is ONE-SHOT: it was consumed by the wake.  A QUIET drive
        // must not fabricate a wake, and the re-registered waker must survive
        // it (a wake with no progress still leaves a registered waker).
        register_rx_udp(handle);
        stack::drive_stack();
        assert_eq!(
            take_wake_log(),
            Vec::<u64>::new(),
            "pin (b): cycle {cycle} — a quiet drive wakes nobody and keeps the waker"
        );

        // Drain the datagram (the conv's re-check observes can_recv and
        // consumes; then the loop would re-register + re-block).
        {
            let mut g = stack::enter_stack();
            let core = g.core.as_mut().expect("pin: stack installed");
            let s = core.sockets.get_mut::<udp::Socket>(handle);
            assert!(s.can_recv(), "pin (b): cycle {cycle} — re-check observes the datagram");
            let mut buf = alloc_vec(64);
            let (n, meta) = s.recv_slice(&mut buf).expect("pin: recv");
            assert_eq!(n, 1);
            assert_eq!(buf[0], cycle as u8);
            assert_eq!(meta.endpoint.addr, IpAddress::Ipv4(PEER));
            assert!(!s.can_recv());
        }
    }
}

// ---------------------------------------------------------------------------
// Pin (c) — deadline arming from poll_at across a retransmit schedule.
// ---------------------------------------------------------------------------

#[test]
fn deadline_arms_from_poll_at_across_retransmit() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();

    // A connect that is never answered: the SYN's retransmit timer is the
    // pending deadline (smoltcp's RTO floor: initial == min == 1000 ms —
    // research-verified against socket/tcp.rs).
    let handle = add_tcp_socket();
    {
        let mut g = stack::enter_stack();
        let core = g.core.as_mut().expect("pin: stack installed");
        let s = core.sockets.get_mut::<tcp::Socket>(handle);
        s.connect(core.iface.context(), (IpAddress::Ipv4(PEER), 1234), 49152).expect("pin: connect");
    }

    stack::drive_stack(); // SYN out, retransmit timer armed
    assert_eq!(tx_count(), 1, "pin (c): the SYN left");
    assert_eq!(
        stack::net_deadline(),
        Some(1000),
        "pin (c): NET_DEADLINE armed from poll_at in tick units (1 tick = 1 ms; RTO initial 1000 ms)"
    );

    // The deadline fires: the tick sweep's check wakes nobody (no waiters
    // registered — pin (g) covers the debt path), so DRIVE directly at the
    // deadline; the retransmit fires and the deadline RE-ARMS strictly
    // later (RFC 6298 backoff — the pin records the shape, not the factor).
    set_ticks(1000);
    stack::drive_stack();
    assert_eq!(tx_count(), 2, "pin (c): the retransmit fired at the deadline");
    let retransmits = tx_tcp(1);
    assert!(retransmits.syn, "pin (c): the retransmit is the SYN again");
    let next = stack::net_deadline().expect("pin (c): re-armed after the retransmit");
    assert!(next > 1000, "pin (c): the re-armed deadline ({next}) is strictly later");
}

// ---------------------------------------------------------------------------
// Pin (d) — the accept-pool demux: two listening pool members on the same
// port; two scripted inbound SYNs land on TWO DISTINCT members (smoltcp's
// accepts() tuple-unset behavior), leaving zero still-Listen members.
// ---------------------------------------------------------------------------

#[test]
fn accept_pool_demuxes_two_syns_to_two_members() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();

    // announce(7) with a test-scaled depth of 2 (the production depth is
    // devip-exact Maxincall = 128 — contract §12.3; the demux SHAPE is
    // depth-independent).
    let mut pool = stack::AcceptPool::announce(2, 7, stack::CoreSel::Loopback).expect("pin: announce");
    assert_eq!(pool.members().len(), 2);
    assert!(pool.all_listening(), "pin (d): the pool starts all-Listen");

    // SYN #1 → absorbed by the first still-Listen member.
    push_rx(tcp4_frame(PEER, GUEST, 1111, 7, 500, None, TcpControl::Syn, &[]));
    stack::drive_stack();
    assert_eq!(pool.listening_count(), 1, "pin (d): one member absorbed the first SYN");

    // SYN #2 (a DIFFERENT remote quad) → absorbed by the OTHER member (the
    // first member's tuple is bound, so its accepts() rejects the new SYN).
    push_rx(tcp4_frame(PEER2, GUEST, 2222, 7, 600, None, TcpControl::Syn, &[]));
    stack::drive_stack();
    assert_eq!(
        pool.listening_count(),
        0,
        "pin (d): two SYNs landed on TWO DISTINCT members — zero still-Listen"
    );

    // The listen-file open's shape: pop the connected members (oldest
    // first), then refill tops the pool back to depth with fresh Listeners.
    let first = pool.take_connected().expect("pin (d): a connected member");
    let second = pool.take_connected().expect("pin (d): a second connected member");
    assert_ne!(first, second, "pin (d): distinct members");
    pool.refill().expect("pin (d): refill");
    assert_eq!(pool.members().len(), 2);
    assert!(pool.all_listening(), "pin (d): refill restores all-Listen");
}

// ---------------------------------------------------------------------------
// Pin (g) — the stack-debt drain: a deadline firing with zero registered
// waiters sets STACK_DEBT; the next drain_debt_if_set() drives the stack
// and clears it.
// ---------------------------------------------------------------------------

#[test]
fn stack_debt_drains_on_next_op() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();

    // The unanswered connect arms the 1000-tick retransmit deadline.
    let handle = add_tcp_socket();
    {
        let mut g = stack::enter_stack();
        let core = g.core.as_mut().expect("pin: stack installed");
        let s = core.sockets.get_mut::<tcp::Socket>(handle);
        s.connect(core.iface.context(), (IpAddress::Ipv4(PEER), 1234), 49152).expect("pin: connect");
    }
    stack::drive_stack();
    assert_eq!(stack::net_deadline(), Some(1000));
    assert_eq!(tx_count(), 1);

    // The deadline fires with ZERO registered waiters (nobody blocked in a
    // devip op): the tick sweep sets the debt instead of waking anyone.
    // (tick_check takes the observed tick count as a parameter — the clock
    // only matters when a DRIVE runs: poll_maintenance compares the
    // retransmit timer against instant_now().)
    stack::tick_check(999);
    assert!(!stack::STACK_DEBT.load(Ordering::Acquire), "pin (g): no debt before the deadline");
    set_ticks(1000); // time reaches the deadline — the drain's drive runs at t=1000
    stack::tick_check(1000);
    assert!(stack::STACK_DEBT.load(Ordering::Acquire), "pin (g): the fired deadline set the debt");
    assert_eq!(take_wake_log(), Vec::<u64>::new(), "pin (g): no waiters to wake");

    // The next devip-dispatched op drains the debt FIRST: the stack drives
    // (the retransmit fires — the TX count bumps) and the flag clears.
    assert!(stack::drain_debt_if_set());
    assert!(!stack::STACK_DEBT.load(Ordering::Acquire), "pin (g): the drain cleared the flag");
    assert_eq!(tx_count(), 2, "pin (g): the draining drive fired the retransmit");
    assert!(stack::net_deadline().expect("pin (g): re-armed") > 1000);

    // A second drain on a clear flag is a no-op.
    assert!(!stack::drain_debt_if_set());
}

// ---------------------------------------------------------------------------
// 08-04 Task 2 — the udp module pins (NETIP-03's second leg + the
// NETIP-02 conv-isolation truth).  The udp proto lands behind these;
// the pin shapes ride the same harness (scripted stack, stub hooks).
// ---------------------------------------------------------------------------

/// Drop-guard: every pin conv-closes what it allocated (a stale conv's
/// SocketHandle names a prior pin's SocketSet — close drops it).
struct ConvGuard(crate::devip::conv::Proto, usize);

impl Drop for ConvGuard {
    fn drop(&mut self) {
        crate::devip::conv::conv_close(self.0, self.1);
    }
}

/// A conv allocation tracked to close at scope end.
fn alloc_udp_conv() -> (usize, ConvGuard) {
    let id = crate::devip::conv::conv_alloc(crate::devip::conv::Proto::Udp)
        .expect("pin: udp clone allocates");
    (id, ConvGuard(crate::devip::conv::Proto::Udp, id))
}

/// The wedged reader's pid (never armed on a waker, never woken — the
/// wedge simulation).
const WEDGED_PID: u64 = 77;

/// The v4-mapped 16-byte form (the devip address model).
fn mapped(a: Ipv4Address) -> [u8; 16] {
    crate::devip::v4prefix_of(&a.octets())
}

/// Read one udp conv's (state, headers, laddr, lport, raddr, rport,
/// announcer, incall len, dataq len) — the pin inspection surface.
fn udp_conv_snapshot(
    id: usize,
) -> (
    crate::devip::conv::ConvState,
    u8,
    [u8; 16],
    u16,
    [u8; 16],
    u16,
    Option<usize>,
    usize,
    usize,
) {
    let node = crate::mcslock::LockNode::new();
    let t = crate::devip::conv::TABLES.lock(&node);
    let c = &t.table(crate::devip::conv::Proto::Udp)[id];
    (
        c.state,
        c.headers,
        c.laddr,
        c.lport,
        c.raddr,
        c.rport,
        c.announcer,
        c.incall.len(),
        c.dataq.len(),
    )
}

/// Pop a spawned conv's data queue head (the pin's delivered-datagram
/// readback).
fn udp_dataq_front(id: usize) -> Option<Vec<u8>> {
    let node = crate::mcslock::LockNode::new();
    let t = crate::devip::conv::TABLES.lock(&node);
    t.table(crate::devip::conv::Proto::Udp)[id].dataq.front().cloned()
}

/// The conv's recorded ttl/tos (the ctl-isolation pin's readback).
fn udp_conv_ttl_tos(id: usize) -> (u32, u32) {
    let node = crate::mcslock::LockNode::new();
    let t = crate::devip::conv::TABLES.lock(&node);
    let c = &t.table(crate::devip::conv::Proto::Udp)[id];
    (c.ttl, c.tos)
}

/// The conv's hangup + errq head (the advise pin's readback).
fn udp_conv_hangup_err(id: usize) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let node = crate::mcslock::LockNode::new();
    let t = crate::devip::conv::TABLES.lock(&node);
    let c = &t.table(crate::devip::conv::Proto::Udp)[id];
    (c.hangup.clone(), c.errq.front().cloned())
}

/// (e) The headers-mode pin (contract §6.2): a scripted inbound datagram
/// to an announced, headers-mode conv reads back as the EXACT 52-byte
/// Udphdr prefix (raddr[16] laddr[16] ifcaddr[16] rport[2] lport[2] —
/// v4 mapped into v6 form, big-endian ports) + the payload, in one
/// datagram-at-once read.
#[test]
fn udp_headers_mode_synthesizes_the_52_byte_prefix() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (id, _guard) = alloc_udp_conv();
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Udp, id, b"announce 7")
        .expect("pin (e): announce");
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Udp, id, b"headers")
        .expect("pin (e): headers");
    let lport = udp_conv_snapshot(id).3;
    assert_eq!(lport, 7, "pin (e): announce bound port 7");
    assert_eq!(udp_conv_snapshot(id).1, 7, "pin (e): headers=7 recorded");

    push_rx(udp4_frame(PEER, GUEST, 4321, 7, b"pin payload"));
    stack::drive_stack();

    let mut buf = [0u8; 128];
    let n = crate::devip::proto::udp_read_data(id, &mut buf).expect("pin (e): read delivers") as usize;
    assert_eq!(n, 52 + 11, "pin (e): one datagram, prefix + payload");
    assert_eq!(&buf[0..16], &mapped(PEER), "pin (e): raddr");
    assert_eq!(&buf[16..32], &mapped(GUEST), "pin (e): laddr (the receiving address)");
    assert_eq!(&buf[32..48], &mapped(GUEST), "pin (e): ifcaddr (the receiving interface's local)");
    assert_eq!(&buf[48..50], &4321u16.to_be_bytes(), "pin (e): rport big-endian");
    assert_eq!(&buf[50..52], &7u16.to_be_bytes(), "pin (e): lport big-endian");
    assert_eq!(&buf[52..n], b"pin payload", "pin (e): the payload verbatim");
}

/// The Fsnewcall pin (contract §6.2): a datagram from a NEW remote quad
/// to an announced (non-headers) conv spawns a new Connected conv
/// carrying the quad + the datagram; the announcer's incall queue holds
/// it until the listen open pops it (the returned chan names the NEW
/// conv's ctl).  A repeat datagram from the same quad lands on the SAME
/// spawned conv; a new quad spawns another.
#[test]
fn udp_fsnewcall_spawns_on_new_remote_quad() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (id, _guard) = alloc_udp_conv();
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Udp, id, b"announce 7")
        .expect("pin (spawn): announce");

    // New quad → spawn.
    push_rx(udp4_frame(PEER, GUEST, 5000, 7, b"one"));
    stack::drive_stack(); // the scripted frame reaches the socket at poll
    crate::devip::proto::udp_rx_drain();
    let snap = udp_conv_snapshot(id);
    assert_eq!(snap.7, 1, "pin (spawn): one incall after the first datagram");
    let nid = {
        let node = crate::mcslock::LockNode::new();
        let t = crate::devip::conv::TABLES.lock(&node);
        t.table(crate::devip::conv::Proto::Udp)[id].incall[0]
    };
    let spawned = udp_conv_snapshot(nid);
    assert_eq!(spawned.0, crate::devip::conv::ConvState::Connected, "pin (spawn): Connected");
    assert_eq!(spawned.4, mapped(PEER), "pin (spawn): raddr is the sender");
    assert_eq!(spawned.5, 5000, "pin (spawn): rport is the sender's");
    assert_eq!(spawned.3, 7, "pin (spawn): lport is the announced port");
    assert_eq!(spawned.6, Some(id), "pin (spawn): the announcer back-link");
    assert_eq!(udp_dataq_front(nid).as_deref(), Some(&b"one"[..]), "pin (spawn): the first datagram delivered");

    // Same quad → the SAME conv (no second spawn).
    push_rx(udp4_frame(PEER, GUEST, 5000, 7, b"two"));
    stack::drive_stack();
    crate::devip::proto::udp_rx_drain();
    assert_eq!(udp_conv_snapshot(id).7, 1, "pin (spawn): no second spawn for a known quad");
    assert_eq!(udp_conv_snapshot(nid).8, 2, "pin (spawn): the repeat datagram queued on the same conv");

    // New quad → another spawn.
    push_rx(udp4_frame(PEER2, GUEST, 5001, 7, b"three"));
    stack::drive_stack();
    crate::devip::proto::udp_rx_drain();
    assert_eq!(udp_conv_snapshot(id).7, 2, "pin (spawn): a second incall for the new quad");

    // The listen open pops the FIRST spawned conv — the returned chan
    // names the NEW conv's ctl (the file-level listen/accept path).
    let lqid = crate::dat::Qid {
        path: crate::devip::qid(crate::devip::conv::Proto::Udp.index(), id as u64, crate::devip::QLISTEN),
        vers: 0,
        typ: crate::dat::QTFILE,
    };
    let lc = crate::dat::Chan::new(crate::dat::DEV_IP, lqid, crate::dat::OREAD);
    let nc = crate::devip::open(&lc, crate::dat::OREAD).expect("pin (spawn): listen open");
    let popped = ((nc.qid.path >> 5) & 0xfff) as usize; // QID(p,c,y): c at bits 5..17
    assert_eq!(popped, nid, "pin (spawn): listen pops the first incoming call");
    assert_eq!(udp_conv_snapshot(id).7, 1, "pin (spawn): the incall queue popped");

    // The spawned conv's own data read delivers the queued datagrams in
    // order (Qmsg datagram-at-once).
    let mut buf = [0u8; 64];
    let n = crate::devip::proto::udp_read_data(nid, &mut buf).expect("pin (spawn): spawned read") as usize;
    assert_eq!(&buf[..n], b"one", "pin (spawn): first datagram first");
    let n = crate::devip::proto::udp_read_data(nid, &mut buf).expect("pin (spawn): spawned read") as usize;
    assert_eq!(&buf[..n], b"two", "pin (spawn): second datagram next");

    // Close out the spawned convs (the announcer's guard covers its own
    // close; incall convs are closed by the announcer's last close).
    // (An earlier `let _ = nc;` dropped the Chan value WITHOUT closing
    // the conv — the popped spawned conv leaked across pins, and a later
    // pin announcing the same port+quad found the stale conv in the
    // drain's per-quad demux instead of spawning fresh.)
    crate::devip::close(&nc);
}

/// The spawned-conv write (WR-01): a listen/accept udp server's reply on
/// the accepted conv's data file rides the ANNOUNCER's socket with the
/// conv's quad as per-write metadata (the donor sends on such convs —
/// the headers-mode write's shape).  Pre-fix the write returned Ok(n)
/// while DROPPING the datagram (success with data loss — the worst
/// failure shape).  A dead/recycled announcer errors loudly, never a
/// wrong-port send on an unrelated socket.
#[test]
fn udp_spawned_conv_writes_ride_the_announcers_socket() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (id, _guard) = alloc_udp_conv();
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Udp, id, b"announce 7")
        .expect("pin (spawnwr): announce");

    // A datagram from a new quad spawns the conv (the Fsnewcall pin's
    // shape).
    push_rx(udp4_frame(PEER, GUEST, 5000, 7, b"ping"));
    stack::drive_stack();
    crate::devip::proto::udp_rx_drain();
    let nid = {
        let node = crate::mcslock::LockNode::new();
        let t = crate::devip::conv::TABLES.lock(&node);
        t.table(crate::devip::conv::Proto::Udp)[id].incall[0]
    };

    // The reply on the spawned conv's data file goes ON THE WIRE with
    // the conv's quad: src GUEST:7 (the announced port wins), dst
    // PEER:5000 (the sender's).
    let base = tx_count();
    let n = crate::devip::proto::udp_write_data(nid, b"pong").expect("pin (spawnwr): the spawned write sends");
    assert_eq!(n, 4, "pin (spawnwr): the full count");
    let (src, dst, sport, dport, payload) = {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (spawnwr): stack installed");
        parse_udp4(core.dev_scripted().tx_log.get(base).expect("pin (spawnwr): a TX frame"))
    };
    assert_eq!((src, dst), (GUEST, PEER), "pin (spawnwr): the reply's addresses");
    assert_eq!((sport, dport), (7, 5000), "pin (spawnwr): the reply's ports — the announced lport wins");
    assert_eq!(payload, b"pong", "pin (spawnwr): the payload verbatim");

    // The dead-announcer arm: pop the spawned conv (the listen open), then
    // close the announcer — the spawned conv's write errors loudly.
    let lqid = crate::dat::Qid {
        path: crate::devip::qid(crate::devip::conv::Proto::Udp.index(), id as u64, crate::devip::QLISTEN),
        vers: 0,
        typ: crate::dat::QTFILE,
    };
    let lc = crate::dat::Chan::new(crate::dat::DEV_IP, lqid, crate::dat::OREAD);
    let nc = crate::devip::open(&lc, crate::dat::OREAD).expect("pin (spawnwr): listen open");
    crate::devip::conv::conv_close(crate::devip::conv::Proto::Udp, id);
    let e = crate::devip::proto::udp_write_data(nid, b"pong")
        .expect_err("pin (spawnwr): a dead announcer errors loudly");
    assert_eq!(e, "udp: no socket on spawned conv", "pin (spawnwr): the loud-error string");
    crate::devip::close(&nc);
}

/// The NETIP-02 isolation pin, part 1: concurrent clone opens allocate
/// INDEPENDENT conversations (distinct conv ids, per-proto tables), and
/// a ctl write on one conversation never appears in another's state.
#[test]
fn udp_conv_isolation_distinct_ids_and_ctl_isolation() {
    let _serial = fresh_scripted_stack();
    let (a, _ga) = alloc_udp_conv();
    let (b, _gb) = alloc_udp_conv();
    assert_ne!(a, b, "pin (isolation): concurrent clones allocate distinct conv ids");
    // Cross-proto independence: an icmp clone never collides with udp's table.
    let icmp_id = crate::devip::conv::conv_alloc(crate::devip::conv::Proto::Icmp)
        .expect("pin (isolation): icmp clone");
    let _gi = ConvGuard(crate::devip::conv::Proto::Icmp, icmp_id);
    assert_eq!(
        udp_conv_snapshot(a).0,
        crate::devip::conv::ConvState::Idle,
        "pin (isolation): fresh udp conv is Idle"
    );

    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Udp, a, b"ttl 123")
        .expect("pin (isolation): ttl on A");
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Udp, b, b"tos 5")
        .expect("pin (isolation): tos on B");
    assert_eq!(udp_conv_ttl_tos(a), (123, 0), "pin (isolation): A carries its own ctl");
    assert_eq!(udp_conv_ttl_tos(b), (255, 5), "pin (isolation): B never sees A's write");
}

/// The NETIP-02 isolation pin, part 2: a wedged reader on conv A
/// (registered, never woken) leaves sibling conv B's data path fully
/// operational — and the wedge's pid is never woken by B's activity.
#[test]
fn udp_wedged_reader_never_blocks_a_sibling_conv() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (a, _ga) = alloc_udp_conv();
    let (b, _gb) = alloc_udp_conv();

    // The wedge: a reader blocked on A forever (registered on the conv +
    // the seam's net-waiter set, never armed on any waker).
    {
        let node = crate::mcslock::LockNode::new();
        let mut t = crate::devip::conv::TABLES.lock(&node);
        t.table_mut(crate::devip::conv::Proto::Udp)[a].waiters.register(WEDGED_PID);
    }
    stack::register_net_waiter(WEDGED_PID);

    // B runs a full round-trip: connect → write (the TX is captured) →
    // scripted reply → read.
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Udp, b, b"connect 10.0.0.2!7000")
        .expect("pin (wedge): B connects");
    let blport = udp_conv_snapshot(b).3;
    assert!(blport != 0, "pin (wedge): connect assigned B a local port");
    crate::devip::proto::udp_write_data(b, b"probe").expect("pin (wedge): B writes");
    assert!(tx_count() >= 1, "pin (wedge): B's datagram left the seam");
    let (src, dst, sport, dport, payload) = {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (wedge): stack installed");
        parse_udp4(core.dev_scripted().tx_log.last().expect("pin (wedge): a TX frame"))
    };
    assert_eq!((src, dst), (GUEST, PEER), "pin (wedge): B's datagram addressed to the peer");
    assert_eq!((sport, dport), (blport, 7000), "pin (wedge): B's quad on the wire");
    assert_eq!(payload, b"probe", "pin (wedge): the payload verbatim");

    push_rx(udp4_frame(PEER, GUEST, 7000, blport, b"reply"));
    stack::drive_stack();
    let mut buf = [0u8; 64];
    let n = crate::devip::proto::udp_read_data(b, &mut buf).expect("pin (wedge): B reads") as usize;
    assert_eq!(&buf[..n], b"reply", "pin (wedge): B's data path fully operational");

    assert!(
        !take_wake_log().contains(&WEDGED_PID),
        "pin (wedge): the wedged reader was never woken by B's activity"
    );
}

/// The advise pin (contract §6.2 + §12.7): an ICMP error associated with
/// the conv qhangups it into the err file unless ignoreadvice — the
/// conv-level wiring pinned host-side (the smoltcp icmp-ingress leg stays
/// unwired per §12.7's listed-but-ungated record).
#[test]
fn udp_advise_hangup_lands_in_errq_unless_ignoreadvice() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (a, _ga) = alloc_udp_conv();
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Udp, a, b"connect 10.0.0.2!7000")
        .expect("pin (advise): A connects");

    crate::devip::proto::udp_advise(a, b"icmp: port unreachable");
    let (hangup, err) = udp_conv_hangup_err(a);
    assert_eq!(hangup.as_deref(), Some(&b"icmp: port unreachable"[..]), "pin (advise): the hangup recorded");
    assert_eq!(err.as_deref(), Some(&b"icmp: port unreachable"[..]), "pin (advise): the err file carries the message");
    // qhangup's semantic: data reads/writes on the conv fail while hung up.
    assert!(crate::devip::proto::udp_read_data(a, &mut [0u8; 16]).is_err(), "pin (advise): read fails");
    assert!(crate::devip::proto::udp_write_data(a, b"x").is_err(), "pin (advise): write fails");

    // ignoreadvice: the advice is dropped entirely.
    let (b, _gb) = alloc_udp_conv();
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Udp, b, b"ignoreadvice")
        .expect("pin (advise): ignoreadvice on B");
    crate::devip::proto::udp_advise(b, b"icmp: whatever");
    let (hangup, err) = udp_conv_hangup_err(b);
    assert!(hangup.is_none() && err.is_none(), "pin (advise): ignoreadvice suppresses the hangup");
}

// ---------------------------------------------------------------------------
// Pins: CR-02 — permanent send failures error instead of wedging the
// writer.  smoltcp 0.14 collapses two PERMANENT failures into the same
// SendError surface as transient TX pressure: an oversized payload
// (PacketBuffer::enqueue's capacity arm returns Full FOREVER) and an
// unaddressable endpoint.  Treated as "buffer full → block", both wedge
// the writer with no wake source (a quiet connection enqueues nothing,
// no send-waker ever fires, poll_at disarms).  The pins assert the
// immediate error (the donor's Etoobig string / the unaddressable
// endpoint error) and that the write NEVER parked.  (RED note: pre-fix
// these pins do not fail — they WEDGE, which is the defect.)
// ---------------------------------------------------------------------------

#[test]
fn udp_write_oversized_or_unaddressable_errors_instead_of_wedging() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (id, _guard) = alloc_udp_conv();
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Udp, id, b"connect 10.0.0.2!7000")
        .expect("pin (cr02): connect");

    // Oversized: one byte past the socket's 64 KiB TX payload buffer
    // (proto.rs's UDP_BUF) can never enqueue — the donor's Etoobig.
    let before = block_count();
    let e = crate::devip::proto::udp_write_data(id, &alloc_vec(64 * 1024 + 1))
        .expect_err("pin (cr02): the oversized write errors");
    assert_eq!(e, "read or write too large", "pin (cr02): the Etoobig string");
    assert_eq!(block_count(), before, "pin (cr02): the oversized write never parked");

    // Unaddressable: a connected conv whose raddr is the v4-mapped
    // unspecified address reaches send_slice with dst 0.0.0.0 →
    // SendError::Unaddressable — permanent, never a block.
    let (id2, _guard2) = alloc_udp_conv();
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Udp, id2, b"connect 0.0.0.0!7000")
        .expect("pin (cr02): the 0.0.0.0 connect binds");
    let before = block_count();
    let e = crate::devip::proto::udp_write_data(id2, b"probe")
        .expect_err("pin (cr02): the unaddressable write errors");
    assert_eq!(e, "devip: unaddressable endpoint", "pin (cr02): the unaddressable string");
    assert_eq!(block_count(), before, "pin (cr02): the unaddressable write never parked");
}

#[test]
fn icmp_write_oversized_or_unaddressable_errors_instead_of_wedging() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (id, _guard) = (crate::devip::conv::conv_alloc(crate::devip::conv::Proto::Icmp)
        .map(|id| (id, ConvGuard(crate::devip::conv::Proto::Icmp, id))))
    .expect("pin (cr02): icmp clone");
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Icmp, id, b"connect 10.0.0.2!7")
        .expect("pin (cr02): connect");

    // Oversized: the write is the 20-byte header area + the ICMP message;
    // one byte of message past the 64 KiB TX buffer (proto.rs's ICMP_BUF)
    // can never enqueue.
    let mut wbuf = alloc_vec(20 + 64 * 1024 + 1);
    wbuf[20] = 8; // EchoRequest (the type byte — cosmetic here)
    let before = block_count();
    let e = crate::devip::proto::icmp_write_data(id, &wbuf)
        .expect_err("pin (cr02): the oversized write errors");
    assert_eq!(e, "read or write too large", "pin (cr02): the Etoobig string");
    assert_eq!(block_count(), before, "pin (cr02): the oversized write never parked");

    // Unaddressable: raddr = the v4-mapped 0.0.0.0.
    let id2 = crate::devip::conv::conv_alloc(crate::devip::conv::Proto::Icmp).expect("pin (cr02): icmp clone 2");
    let _g2 = ConvGuard(crate::devip::conv::Proto::Icmp, id2);
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Icmp, id2, b"connect 0.0.0.0!7")
        .expect("pin (cr02): the 0.0.0.0 connect binds");
    let mut wbuf = alloc_vec(20 + 8);
    wbuf[20] = 8;
    let before = block_count();
    let e = crate::devip::proto::icmp_write_data(id2, &wbuf)
        .expect_err("pin (cr02): the unaddressable write errors");
    assert_eq!(e, "devip: unaddressable endpoint", "pin (cr02): the unaddressable string");
    assert_eq!(block_count(), before, "pin (cr02): the unaddressable write never parked");
}

#[test]
fn icmp6_write_oversized_errors_instead_of_wedging() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let id = crate::devip::conv::conv_alloc(crate::devip::conv::Proto::Icmp6).expect("pin (cr02): icmpv6 clone");
    let _guard = ConvGuard(crate::devip::conv::Proto::Icmp6, id);
    crate::devip::proto::ctl_write(crate::devip::conv::Proto::Icmp6, id, b"connect fec0::2!1")
        .expect("pin (cr02): connect");

    // Oversized: the write is the 40-byte IP6 header area + the message;
    // one byte of message past the 64 KiB TX buffer can never enqueue.
    let mut wbuf = alloc_vec(40 + 64 * 1024 + 1);
    wbuf[40] = 128; // EchoRequest (cosmetic)
    let before = block_count();
    let e = crate::devip::proto::icmp6_write_data(id, &wbuf)
        .expect_err("pin (cr02): the oversized write errors");
    assert_eq!(e, "read or write too large", "pin (cr02): the Etoobig string");
    assert_eq!(block_count(), before, "pin (cr02): the oversized write never parked");
}

// ---------------------------------------------------------------------------
// 08-05 Task 1 — the tcp module pins (NETIP-03's third leg; the contract
// doc's TCP chapter made executable).  Pinned here BEFORE the guest gate:
//
// - `tcp_state_rename_table_is_devip_exact` — the 11-state 1:1 rename
//   (smoltcp tcp::State ↔ devip tcpstates[]), every arm.
// - `tcp_connect_blocks_until_established_then_connected` — the P-K bridge:
//   the ctl connect PARKS (the block hook recorded it) until the scripted
//   SYN-ACK — injected from INSIDE the park — drives the socket to
//   Established → Connected.
// - `tcp_connect_rst_mid_syn_maps_econrefused` — a scripted RST to the SYN
//   closes the socket mid-SYN → the write fails "connection refused" and
//   the conv takes the donor's Fsconnected failure shape.
// - `tcp_connect_deadline_expiry_maps_etimedout` — the bridge's own
//   deadline (smoltcp retransmits forever) → "connection timed out".
// - `tcp_announce_builds_contract_depth_pool_and_listen_pops_oldest` — the
//   P-A accept pool: announce builds 128 listeners (contract §12.3), two
//   scripted SYNs occupy two members, the listen-file open pops the oldest
//   connected member as the NEW conv, the pool refills, a non-announced
//   conv errors "not announced".
// - `tcp_listen_hangup_during_parked_wait_errors` — a hangup ctl on the
//   announced conv wakes the parked listen open with "listen hungup".
// - `tcp_close_hangup_hangupxmit_wire_shapes` — P-L: close → FIN,
//   hangup → RST, hangupxmit → FIN sequenced AFTER the queued TX data.
// - `tcp_full_exchange_moves_the_stats_counters` — P-M: a full scripted
//   echo exchange moves ActiveOpens/PassiveOpens/InSegs/OutSegs; the stats
//   file is the 28-tag contract format; the status line is §5.2's shape.
// - `tcp_checksum_and_porthog_are_accepted_and_recorded` — OQ-4: the
//   checksum/tcpporthogdefense verbs succeed, record, and netlog-note;
//   keepalive requires Established (the donor's typo'd string verbatim).
//
// THE SINGLE-THREADED SHAPE: the blocking file paths run on the test
// thread; the scripted peer's answer is injected from INSIDE the park via
// the block hook (ON_BLOCK — a one-shot; BLOCK_RELAY — the standing
// scripted-device loopback relay).  The host's std::thread shape was tried
// and REJECTED: two host threads genuinely race the seam's BusyCell (the
// guest's single-CPU IRQ-masked invariant makes it uncontended there; the
// fail-fast panic fired here).  The on-block injection exercises the SAME
// blocking paths with the park recorded by BLOCK_COUNT.
// ---------------------------------------------------------------------------

use crate::devip::conv::{ConvState, Proto};
use crate::devip::proto;

/// A one-shot action run from inside the block hook (see the section
/// header): the scripted peer's answer lands WHILE the file path is
/// parked — the block hook's invocation IS the park proof.
static ON_BLOCK: Mutex<Option<Box<dyn FnOnce() + Send>>> = Mutex::new(None);

/// When set, every block relays newly captured TX frames back as RX (the
/// scripted device CAPTURES, never loops — the guest's LoopbackDev loops
/// internally; this flag makes the scripted device behave like it for the
/// full-exchange pin).
static BLOCK_RELAY: AtomicBool = AtomicBool::new(false);

/// The relay cursor for the scripted-device loopback (the fresh install
/// resets it — the fresh device's tx_log starts empty).
static RELAYED: AtomicU64 = AtomicU64::new(0);

/// Relay newly captured TX frames back as scripted RX (idempotent).
fn relay_new_tx() {
    let mut g = stack::enter_stack();
    let core = g.core.as_mut().expect("pin: stack installed");
    let dev = core.dev_scripted_mut();
    let from = RELAYED.load(Ordering::Relaxed) as usize;
    if dev.tx_log.len() > from {
        let new: Vec<Vec<u8>> = dev.tx_log[from..].to_vec();
        RELAYED.store(dev.tx_log.len() as u64, Ordering::Relaxed);
        for f in new {
            dev.push_rx(f);
        }
    }
}

/// Install the one-shot on-block action (the scripted answer).
fn set_on_block(act: impl FnOnce() + Send + 'static) {
    *ON_BLOCK.lock().unwrap_or_else(|e| e.into_inner()) = Some(Box::new(act));
}

/// A tcp conv allocation tracked to close at scope end.
fn alloc_tcp_conv() -> (usize, ConvGuard) {
    let id = crate::devip::conv::conv_alloc(Proto::Tcp).expect("pin: tcp clone allocates");
    (id, ConvGuard(Proto::Tcp, id))
}

/// Snapshot a tcp conv for the pins: (state, laddr, lport, raddr, rport,
/// hangup-present, pool depth, estab-marked).
#[allow(clippy::type_complexity)]
fn tcp_conv_snapshot(id: usize) -> (ConvState, [u8; 16], u16, [u8; 16], u16, bool, Option<usize>, bool) {
    let node = crate::mcslock::LockNode::new();
    let t = crate::devip::conv::TABLES.lock(&node);
    let c = &t.table(Proto::Tcp)[id];
    (
        c.state,
        c.laddr,
        c.lport,
        c.raddr,
        c.rport,
        c.hangup.is_some(),
        c.pool.as_ref().map(|p| p.members().len()),
        c.estab,
    )
}

/// The announcing conv's pool member count still in Listen (None = no pool).
fn tcp_pool_listening(id: usize) -> Option<usize> {
    let node = crate::mcslock::LockNode::new();
    let t = crate::devip::conv::TABLES.lock(&node);
    t.table(Proto::Tcp)[id].pool.as_ref().map(|p| p.listening_count())
}

/// The live smoltcp state of the conv's socket (None = socket-less).
fn tcp_sock_state(id: usize) -> Option<tcp::State> {
    let node = crate::mcslock::LockNode::new();
    let t = crate::devip::conv::TABLES.lock(&node);
    let h = t.table(Proto::Tcp)[id].sock?;
    let g = stack::enter_stack();
    let core = g.core.as_ref().expect("pin: stack installed");
    Some(core.sockets.get::<tcp::Socket>(h).state())
}

/// The listen-file chan for an announced tcp conv (the spawn pin's shape).
fn tcp_listen_chan(id: usize) -> crate::dat::Chan {
    let q = crate::devip::qid(Proto::Tcp.index(), id as u64, crate::devip::QLISTEN);
    crate::dat::Chan::new(
        crate::dat::DEV_IP,
        crate::dat::Qid { path: q, vers: 0, typ: crate::dat::QTFILE },
        crate::dat::OREAD,
    )
}

/// The scripted peer's ISN for the connect-bridge pins.
const PEER_ISN: i32 = 7000;

/// Drive a conv-level `connect <PEER!port>` through the scripted peer:
/// the SYN-ACK is injected from inside the connect's own first block (the
/// park proof — the answer cannot land until the file path blocks).
/// Returns the local ISN parsed from the captured SYN.
fn scripted_connect(id: usize, remote_port: u16) -> i32 {
    let base = tx_count();
    let before = block_count();
    set_on_block(move || {
        // The connect's first drive emitted the SYN; the block means the
        // path PARKED — answer it now.
        let syn = tx_tcp(tx_count() - 1);
        assert!(syn.syn && syn.ack.is_none(), "pin (connect): the SYN left before the park's answer");
        push_rx(tcp4_frame(PEER, GUEST, remote_port, syn.sport, PEER_ISN, Some(syn.seq + 1), TcpControl::Syn, &[]));
    });
    let cmd = format!("connect 10.0.0.2!{remote_port}").into_bytes();
    proto::ctl_write(Proto::Tcp, id, &cmd).expect("pin (connect): the ctl connect succeeds");
    assert!(
        block_count() > before,
        "pin (connect): the connect BLOCKED — the answer fired from inside the park (P-K)"
    );
    let syn = (base..tx_count())
        .map(tx_tcp)
        .find(|l| l.syn && l.ack.is_none() && l.dport == remote_port)
        .expect("pin (connect): the captured SYN");
    assert_eq!((syn.src, syn.dst), (GUEST, PEER), "pin (connect): the SYN's addresses");
    syn.seq
}

// ---------------------------------------------------------------------------
// Pin: the state rename table (contract §5.2 — every arm, a pure 1:1 rename).
// ---------------------------------------------------------------------------

#[test]
fn tcp_state_rename_table_is_devip_exact() {
    assert_eq!(proto::tcp_state_name(tcp::State::Closed), b"Closed");
    assert_eq!(proto::tcp_state_name(tcp::State::Listen), b"Listen");
    assert_eq!(proto::tcp_state_name(tcp::State::SynSent), b"Syn_sent");
    assert_eq!(proto::tcp_state_name(tcp::State::SynReceived), b"Syn_received");
    assert_eq!(proto::tcp_state_name(tcp::State::Established), b"Established");
    assert_eq!(proto::tcp_state_name(tcp::State::FinWait1), b"Finwait1");
    assert_eq!(proto::tcp_state_name(tcp::State::FinWait2), b"Finwait2");
    assert_eq!(proto::tcp_state_name(tcp::State::CloseWait), b"Close_wait");
    assert_eq!(proto::tcp_state_name(tcp::State::Closing), b"Closing");
    assert_eq!(proto::tcp_state_name(tcp::State::LastAck), b"Last_ack");
    assert_eq!(proto::tcp_state_name(tcp::State::TimeWait), b"Time_wait");
}

// ---------------------------------------------------------------------------
// Pin: the P-K blocking-connect bridge.
// ---------------------------------------------------------------------------

#[test]
fn tcp_connect_blocks_until_established_then_connected() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (id, _guard) = alloc_tcp_conv();
    scripted_connect(id, 1234);
    let snap = tcp_conv_snapshot(id);
    assert_eq!(snap.0, ConvState::Connected, "pin (connect): Connected at the file level");
    assert_eq!(snap.3, mapped(PEER), "pin (connect): raddr recorded");
    assert_eq!(snap.4, 1234, "pin (connect): rport recorded");
    assert!(snap.2 != 0, "pin (connect): a local port was assigned");
    assert!(snap.7, "pin (connect): the bridge observed Established (CurrEstab bookkeeping)");
    assert_eq!(tcp_sock_state(id), Some(tcp::State::Established));
    // The seam's wake fired for the parked connecter (the waker bridge —
    // set_state(Established) on the SYN-ACK ingest wakes the recv waker).
    assert!(take_wake_log().contains(&STUB_PID), "pin (connect): the waker bridge woke the parker");
}

#[test]
fn tcp_connect_rst_mid_syn_maps_econrefused() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (id, _guard) = alloc_tcp_conv();
    let base = tx_count();
    set_on_block(move || {
        let syn = tx_tcp(tx_count() - 1);
        assert!(syn.syn, "pin (rst): the SYN left before the park's answer");
        // RST the SYN (RST|ACK acking isn+1 — smoltcp's SynSent RST
        // acceptance).
        push_rx(tcp4_frame(PEER, GUEST, 1234, syn.sport, 0, Some(syn.seq + 1), TcpControl::Rst, &[]));
    });
    let err = proto::ctl_write(Proto::Tcp, id, b"connect 10.0.0.2!1234").err().expect("pin (rst): the error");
    assert_eq!(err, "connection refused", "pin (rst): the donor's Econrefused mapping");
    // The donor's Fsconnected(c, reason) shape: the conv is left Connected
    // at the file level with the socket dead and the reason hung on it
    // (the err file carries the reason per contract §3).
    let snap = tcp_conv_snapshot(id);
    assert_eq!(snap.0, ConvState::Connected, "pin (rst): the Fsconnected failure shape");
    assert!(snap.5, "pin (rst): the hangup recorded");
    assert!(proto::tcp_read_data(id, &mut [0u8; 8]).is_err(), "pin (rst): reads error on the hung conv");
    assert!(proto::tcp_write_data(id, b"x").is_err(), "pin (rst): writes error on the hung conv");
    let _ = base;
}

#[test]
fn tcp_connect_deadline_expiry_maps_etimedout() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (id, _guard) = alloc_tcp_conv();
    set_on_block(move || {
        // Fast-forward past the bridge's connect deadline (smoltcp
        // retransmits forever — the Etimedout mapping is the bridge's own
        // deadline); the tick sweep's wake is the guest's wake path for
        // the parked waiter (exercised here for shape fidelity).
        set_ticks(120_001);
        stack::tick_check(120_001);
    });
    let err = proto::ctl_write(Proto::Tcp, id, b"connect 10.0.0.2!1234").err().expect("pin (timeout): the error");
    assert_eq!(err, "connection timed out", "pin (timeout): the donor's Etimedout mapping");
    let snap = tcp_conv_snapshot(id);
    assert!(snap.5, "pin (timeout): the reason hung on the conv");
    assert_eq!(tcp_sock_state(id), None, "pin (timeout): the half-open socket is gone");
}

// ---------------------------------------------------------------------------
// Pin: the P-A accept pool at the conv level (announce → SYNs → listen pop).
// ---------------------------------------------------------------------------

#[test]
fn tcp_announce_builds_contract_depth_pool_and_listen_pops_oldest() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (id, _guard) = alloc_tcp_conv();
    proto::ctl_write(Proto::Tcp, id, b"announce 7").expect("pin (pool): announce");
    let snap = tcp_conv_snapshot(id);
    assert_eq!(snap.0, ConvState::Announced, "pin (pool): Announced");
    assert_eq!(snap.2, 7, "pin (pool): the announced port");
    assert_eq!(
        snap.6,
        Some(stack::ACCEPT_POOL_DEPTH),
        "pin (pool): the pool builds at the contract-doc depth (128, §12.3)"
    );
    assert_eq!(tcp_pool_listening(id), Some(128), "pin (pool): all members Listen");

    // Two scripted SYNs occupy TWO members (pin (d)'s demux shape, here
    // through the conv announce).
    push_rx(tcp4_frame(PEER, GUEST, 1111, 7, 500, None, TcpControl::Syn, &[]));
    push_rx(tcp4_frame(PEER2, GUEST, 2222, 7, 600, None, TcpControl::Syn, &[]));
    stack::drive_stack();
    assert_eq!(tcp_pool_listening(id), Some(126), "pin (pool): two SYNs occupied two members");

    // The listen-file open pops the OLDEST connected member as the NEW
    // conv (the returned chan names the new conv's ctl).
    let nc = crate::devip::open(&tcp_listen_chan(id), crate::dat::OREAD).expect("pin (pool): listen open");
    let nid = ((nc.qid.path >> 5) & 0xfff) as usize;
    assert_ne!(nid, id, "pin (pool): the pop materialized a NEW conv");
    let nsnap = tcp_conv_snapshot(nid);
    assert_eq!(nsnap.0, ConvState::Connected, "pin (pool): the accepted conv is Connected");
    assert_eq!(nsnap.3, mapped(PEER), "pin (pool): the oldest call's raddr");
    assert_eq!(nsnap.4, 1111, "pin (pool): the oldest call's rport");
    assert_eq!(nsnap.2, 7, "pin (pool): the accepted conv's lport is the announced one");
    // The pool refilled to depth — with the pending second call still
    // occupying its member: 128 members, 127 Listen + 1 SynReceived.
    assert_eq!(tcp_conv_snapshot(id).6, Some(128), "pin (pool): refilled to depth");
    assert_eq!(tcp_pool_listening(id), Some(127), "pin (pool): refill + the pending second call");
    // The second SYN's member pops next; its refill tops the pool to
    // 128 all-Listen.
    let nc2 = crate::devip::open(&tcp_listen_chan(id), crate::dat::OREAD).expect("pin (pool): second listen");
    let nid2 = ((nc2.qid.path >> 5) & 0xfff) as usize;
    assert_eq!(tcp_conv_snapshot(nid2).3, mapped(PEER2), "pin (pool): the second call's quad");
    assert_eq!(tcp_conv_snapshot(id).6, Some(128), "pin (pool): depth after the second refill");
    assert_eq!(tcp_pool_listening(id), Some(128), "pin (pool): all-Listen after both pops");
    crate::devip::close(&nc);
    crate::devip::close(&nc2);

    // A listen open against a non-announced conv errors "not announced".
    let (other, _g2) = alloc_tcp_conv();
    let err = crate::devip::open(&tcp_listen_chan(other), crate::dat::OREAD).err().expect("pin (pool): the error");
    assert_eq!(err, "not announced", "pin (pool): the donor's listen gate");
}

#[test]
fn tcp_listen_hangup_during_parked_wait_errors() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (id, _guard) = alloc_tcp_conv();
    proto::ctl_write(Proto::Tcp, id, b"announce 8").expect("pin (listen-hangup): announce");
    let before = block_count();
    // The listen parks (no incoming call); the hangup ctl lands from
    // INSIDE the park — the donor's qisclosed(cv->rq) arm errors the
    // sleeper "listen hungup".
    set_on_block(move || {
        proto::ctl_write(Proto::Tcp, id, b"hangup").expect("pin (listen-hangup): the hangup ctl");
    });
    let err = crate::devip::open(&tcp_listen_chan(id), crate::dat::OREAD).err().expect("pin (listen-hangup): the error");
    assert_eq!(err, "listen hungup", "pin (listen-hangup): the donor's error");
    assert!(block_count() > before, "pin (listen-hangup): the listen PARKED before the hangup");
    assert!(take_wake_log().contains(&STUB_PID), "pin (listen-hangup): the hangup woke the parker");
}

// ---------------------------------------------------------------------------
// Pin: the P-L close/hangup/hangupxmit wire shapes.
// ---------------------------------------------------------------------------

#[test]
fn tcp_close_hangup_hangupxmit_wire_shapes() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();

    // `close` → a FIN on the wire (smoltcp close()).
    let (a, _ga) = alloc_tcp_conv();
    let isn_a = scripted_connect(a, 1234);
    let base = tx_count();
    proto::ctl_write(Proto::Tcp, a, b"close").expect("pin (close): the verb");
    stack::drive_stack();
    let fin = (base..tx_count())
        .map(tx_tcp)
        .find(|l| l.fin)
        .expect("pin (close): a FIN left after the verb");
    assert_eq!(fin.seq, isn_a + 1, "pin (close): the FIN follows the handshake sequence");
    assert_eq!(tcp_sock_state(a), Some(tcp::State::FinWait1));

    // `hangup` → an RST on the wire (smoltcp abort(): "one reset packet
    // will be sent" — socket/tcp.rs).
    let (b, _gb) = alloc_tcp_conv();
    scripted_connect(b, 1235);
    let base = tx_count();
    proto::ctl_write(Proto::Tcp, b, b"hangup").expect("pin (hangup): the verb");
    stack::drive_stack();
    assert!(
        (base..tx_count()).map(tx_tcp).any(|l| l.rst),
        "pin (hangup): an RST left after the verb"
    );
    assert_eq!(tcp_sock_state(b), Some(tcp::State::Closed));

    // `hangupxmit` → close-after-TX-drain (P-L): with unacked data queued,
    // the FIN's sequence number comes AFTER the data (smoltcp's egress
    // emits FIN only once the segment covers the buffer's tail).
    let (c, _gc) = alloc_tcp_conv();
    let isn_c = scripted_connect(c, 1237);
    let base = tx_count(); // the data frame rides this window too
    proto::tcp_write_data(c, b"drain-me").expect("pin (hangupxmit): the write queues");
    proto::ctl_write(Proto::Tcp, c, b"hangupxmit").expect("pin (hangupxmit): the verb");
    stack::drive_stack();
    let legs: Vec<TcpLeg> = (base..tx_count()).map(tx_tcp).collect();
    let fin = legs.iter().find(|l| l.fin).expect("pin (hangupxmit): a FIN left");
    assert_eq!(
        fin.seq,
        isn_c + 1 + 8,
        "pin (hangupxmit): the FIN is sequenced AFTER the 8 queued bytes (close-after-drain)"
    );
    // The queued data itself went out before the FIN's position.
    assert!(
        legs.iter().any(|l| l.payload == b"drain-me"),
        "pin (hangupxmit): the queued data was transmitted"
    );
}

// ---------------------------------------------------------------------------
// Pin: the LAST-close FIN delivery (CR-01 — conv_close's lingering socket).
// smoltcp's close() only TRANSITIONS the state machine (FIN queued
// internally): removing the socket from the set in the same breath destroys
// the FIN before any poll can emit it (the peer never sees EOF) and drops
// any queued-but-unsent TX tail.  The donor's tcb survives closeconv until
// the close handshake completes/ages out; the seam's linger list is that
// shape: the closed conv's socket stays IN the SocketSet until
// State::Closed, then the drive reaps it.
// ---------------------------------------------------------------------------

#[test]
fn tcp_conv_close_lingers_the_socket_until_closed() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let (id, _guard) = alloc_tcp_conv();
    let isn = scripted_connect(id, 1240);
    let base = tx_count();

    // The LAST close (the fd path — Harvey programs close fds on exit;
    // distinct from the `close` VERB pinned above): the FIN must leave,
    // and the socket must LINGER in the seam mid-close.
    crate::devip::conv::conv_close(Proto::Tcp, id);
    let fin = (base..tx_count())
        .map(tx_tcp)
        .find(|l| l.fin)
        .expect("pin (linger): the last close emits the FIN");
    assert_eq!(fin.seq, isn + 1, "pin (linger): the FIN follows the handshake sequence");
    let handle = {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (linger): stack installed");
        assert_eq!(core.linger.len(), 1, "pin (linger): the closed conv's socket lingers");
        let h = core.linger[0];
        assert_eq!(
            core.sockets.get::<tcp::Socket>(h).state(),
            tcp::State::FinWait1,
            "pin (linger): the lingered socket is mid-close, not destroyed"
        );
        h
    };

    // The peer ACKs our FIN and closes its side too: the close handshake
    // completes (our ACK leaves), the socket lands in TimeWait — still
    // parked (the donor's tcb frees only when the close AGES OUT).
    push_rx(tcp4_frame(PEER, GUEST, 1240, fin.sport, PEER_ISN + 1, Some(isn + 2), TcpControl::Fin, &[]));
    stack::drive_stack();
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (linger): stack installed");
        assert_eq!(
            core.sockets.get::<tcp::Socket>(handle).state(),
            tcp::State::TimeWait,
            "pin (linger): the completed handshake lands in TimeWait"
        );
        assert_eq!(core.linger.len(), 1, "pin (linger): TimeWait still lingers");
    }

    // The close timer ages the socket out (smoltcp's CLOSE_DELAY = 10 s,
    // driven by the seam's poll legs — the stub clock jumps it): the next
    // drive reaps the socket from the set entirely.
    set_ticks(10_001);
    stack::drive_stack();
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (linger): stack installed");
        assert!(core.linger.is_empty(), "pin (linger): the Closed socket is reaped");
        assert_eq!(core.sockets.iter().count(), 0, "pin (linger): the SocketSet slot frees");
    }
}

// ---------------------------------------------------------------------------
// Pin: CR-06 — the tcp gc under clone pressure force-HANGS-UP stale convs
// (the donor's tcpgc → localclose: qhangup + wake, slot ownership
// unchanged) and the cloner gets Enodev; an in-use slot is NEVER reset and
// handed out (that aliased the victim's open fds onto the cloner's fresh
// conversation — reads/writes crossing conversations, and the victim's
// eventual close resetting the live conv out from under its owner).
// ---------------------------------------------------------------------------

#[test]
fn tcp_gc_hangs_up_stale_convs_but_never_steals_an_in_use_slot() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    // Fill the 16-slot table: each conv connected to the scripted peer,
    // then hangup-aborted — the socket fully Closed with inuse = 1 (the
    // common "read EOF, fd not yet closed" shape).
    let mut guards = Vec::new();
    for i in 0..crate::devip::conv::NCONV {
        let (id, g) = alloc_tcp_conv();
        scripted_connect(id, 1400 + i as u16);
        proto::ctl_write(Proto::Tcp, id, b"hangup").expect("pin (gc): hangup aborts");
        assert_eq!(tcp_sock_state(id), Some(tcp::State::Closed), "pin (gc): the aborted socket is Closed");
        guards.push(g);
    }
    // Clone pressure: the gc hangs the stale convs up (never steals) and
    // the retried scan still finds every slot in use → Enodev.
    let e = crate::devip::conv::conv_alloc(Proto::Tcp).expect_err("pin (gc): a full table is Enodev");
    assert_eq!(e, "no free devices", "pin (gc): the donor's exhaustion error");
    // The victim's conversation survives the gc (no reset-and-hand-out)
    // with the hangup recorded (localclose's qhangup).
    let snap = tcp_conv_snapshot(0);
    assert_eq!(snap.0, ConvState::Connected, "pin (gc): the victim's state survives the gc");
    assert_eq!(snap.3, mapped(PEER), "pin (gc): the victim's quad survives the gc");
    assert!(snap.5, "pin (gc): the gc's hangup marker stands");
    // The slot frees at the OWNER's close: close conv 3, re-clone, get 3.
    crate::devip::conv::conv_close(Proto::Tcp, 3);
    let id = crate::devip::conv::conv_alloc(Proto::Tcp).expect("pin (gc): the owner's close freed the slot");
    assert_eq!(id, 3, "pin (gc): the freed slot is the reusable one");
    guards.push(ConvGuard(Proto::Tcp, id));
}

// ---------------------------------------------------------------------------
// Pin: P-M stats honesty — a full scripted echo exchange moves the bridge-
// maintained counters; the stats file is the contract's 28-tag format; the
// status line carries a devip state name.  The scripted device CAPTURES TX
// (never loops), so BLOCK_RELAY makes every block relay new captures back
// as RX — the handshake and the data legs complete inside the real
// blocking paths.
// ---------------------------------------------------------------------------

#[test]
fn tcp_full_exchange_moves_the_stats_counters() {
    let _serial = fresh_scripted_stack();
    RELAYED.store(0, Ordering::Relaxed);
    set_guest_addr();
    let base_active = proto::TCP_STATS[proto::TCPSTAT_ACTIVE_OPENS].load(Ordering::Relaxed);
    let base_passive = proto::TCP_STATS[proto::TCPSTAT_PASSIVE_OPENS].load(Ordering::Relaxed);
    let base_in = proto::TCP_STATS[proto::TCPSTAT_IN_SEGS].load(Ordering::Relaxed);
    let base_out = proto::TCP_STATS[proto::TCPSTAT_OUT_SEGS].load(Ordering::Relaxed);

    // The server announces; the client connects through the bridge with
    // the block-time relay carrying every leg (SYN → the pool member;
    // SYN-ACK → the client; ACK → the member: Established both ends).
    BLOCK_RELAY.store(true, Ordering::Relaxed);
    let (sid, _gs) = alloc_tcp_conv();
    proto::ctl_write(Proto::Tcp, sid, b"announce 9").expect("pin (stats): announce");
    let (cid, _gc) = alloc_tcp_conv();
    proto::ctl_write(Proto::Tcp, cid, b"connect 10.0.0.1!9").expect("pin (stats): the connect completes over the relay");
    // The client's ACK may sit captured-but-unrelayed when the connect
    // returns; one explicit relay+drive settles the member at Established.
    relay_new_tx();
    stack::drive_stack();

    // The listen open pops the Established member as the new conv.
    let nc = crate::devip::open(&tcp_listen_chan(sid), crate::dat::OREAD).expect("pin (stats): listen pops");
    let nid = ((nc.qid.path >> 5) & 0xfff) as usize;
    assert_eq!(tcp_sock_state(nid), Some(tcp::State::Established), "pin (stats): the accepted conv is Established");

    // The echo exchange — the reads' own pre-block drives + the block-time
    // relay move the payloads; both ends use the REAL blocking file paths.
    proto::tcp_write_data(cid, b"ping").expect("pin (stats): client write");
    let mut buf = [0u8; 64];
    let n = proto::tcp_read_data(nid, &mut buf).expect("pin (stats): server read") as usize;
    assert_eq!(&buf[..n], b"ping", "pin (stats): the server got the payload");
    proto::tcp_write_data(nid, b"pong").expect("pin (stats): server write");
    let n = proto::tcp_read_data(cid, &mut buf).expect("pin (stats): client read") as usize;
    assert_eq!(&buf[..n], b"pong", "pin (stats): the client got the echo");
    BLOCK_RELAY.store(false, Ordering::Relaxed);

    // The bridge-observed counters moved (P-M: no zeros-forever stats file).
    assert_eq!(
        proto::TCP_STATS[proto::TCPSTAT_ACTIVE_OPENS].load(Ordering::Relaxed),
        base_active + 1,
        "pin (stats): ActiveOpens"
    );
    assert_eq!(
        proto::TCP_STATS[proto::TCPSTAT_PASSIVE_OPENS].load(Ordering::Relaxed),
        base_passive + 1,
        "pin (stats): PassiveOpens"
    );
    assert_eq!(
        proto::TCP_STATS[proto::TCPSTAT_IN_SEGS].load(Ordering::Relaxed),
        base_in + 2,
        "pin (stats): InSegs (one per recv completion)"
    );
    assert_eq!(
        proto::TCP_STATS[proto::TCPSTAT_OUT_SEGS].load(Ordering::Relaxed),
        base_out + 2,
        "pin (stats): OutSegs (one per send completion)"
    );

    // The stats file: the contract's 28 tagged "Name: N" lines (§5.3).
    let stats = proto::tcp_stats();
    let text = std::string::String::from_utf8(stats).expect("pin (stats): utf8");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 28, "pin (stats): the 28-tag contract format");
    assert!(lines.iter().all(|l| l.contains(": ")), "pin (stats): every line tagged");
    for tag in ["ActiveOpens", "PassiveOpens", "CurrEstab", "InSegs", "OutSegs", "OutRsts"] {
        assert!(text.contains(&format!("{tag}: ")), "pin (stats): the {tag} line");
    }

    // The status line: the contract's §5.2 format with a devip state name
    // (the zero-mapped tail per the contract's field-mapping rule).
    let status = proto::status(Proto::Tcp, cid);
    let status = std::string::String::from_utf8(status).expect("pin (stats): status utf8");
    assert!(status.starts_with("Established qin "), "pin (stats): the state name leads, got {status:?}");
    assert!(
        status.ends_with(
            " rq 0.0 srtt 0 mdev 0 sst 0 cwin 0 swin 0>>0 rwin 0>>0 qscale 0 timer.start 0 timer.count 0 rerecv 0 katimer.start 0 katimer.count 0\n"
        ),
        "pin (stats): the contract's zero-mapped tail, got {status:?}"
    );
    crate::devip::close(&nc);
}

// ---------------------------------------------------------------------------
// Pin: OQ-4 — checksum/tcpporthogdefense are accepted-and-recorded with a
// netlog note (no smoltcp mechanism; contract §5.1), keepalive requires
// Established, and the tcp ctl fallthrough errors the contract's strings.
// ---------------------------------------------------------------------------

#[test]
fn tcp_checksum_and_porthog_are_accepted_and_recorded() {
    let _serial = fresh_scripted_stack();
    let (id, _g) = alloc_tcp_conv();

    // The netlog flag gates the note: set → the notes land in the queue.
    crate::devip::netlog::ctl(b"set tcp").expect("pin (oq4): set tcp");
    proto::ctl_write(Proto::Tcp, id, b"checksum 0").expect("pin (oq4): checksum accepted");
    proto::ctl_write(Proto::Tcp, id, b"tcpporthogdefense on").expect("pin (oq4): porthog on accepted");
    {
        let node = crate::mcslock::LockNode::new();
        let t = crate::devip::conv::TABLES.lock(&node);
        assert!(t.table(Proto::Tcp)[id].nochecksum, "pin (oq4): checksum 0 recorded on the conv");
    }
    let lines = crate::devip::netlog::drain_lines();
    assert_eq!(lines.len(), 2, "pin (oq4): both verbs noted under the tcp flag");
    assert!(lines[0].windows(8).any(|w| w == b"checksum"), "pin (oq4): the checksum note");
    assert!(lines[1].windows(17).any(|w| w == b"tcpporthogdefense"), "pin (oq4): the porthog note");

    // Flag cleared → the verbs stay accepted-and-recorded but no note emits.
    crate::devip::netlog::ctl(b"clear tcp").expect("pin (oq4): clear tcp");
    proto::ctl_write(Proto::Tcp, id, b"checksum 1").expect("pin (oq4): checksum 1 accepted");
    {
        let node = crate::mcslock::LockNode::new();
        let t = crate::devip::conv::TABLES.lock(&node);
        assert!(!t.table(Proto::Tcp)[id].nochecksum, "pin (oq4): checksum 1 recorded");
    }
    assert!(crate::devip::netlog::drain_lines().is_empty(), "pin (oq4): gated when clear");

    // keepalive requires the Established state — the donor's typo verbatim.
    assert_eq!(
        proto::ctl_write(Proto::Tcp, id, b"keepalive").err().expect("pin (oq4): the error"),
        "connection must be in Establised state",
        "pin (oq4): the keepalive state gate"
    );
    // The ctl fallthrough strings (contract §5.1/§11).
    assert_eq!(
        proto::ctl_write(Proto::Tcp, id, b"tcpporthogdefense maybe").err().expect("pin (oq4): the error"),
        "unknown value for tcpporthogdefense"
    );
    assert_eq!(
        proto::ctl_write(Proto::Tcp, id, b"frobnicate").err().expect("pin (oq4): the error"),
        "unknown control request"
    );
}

// ---------------------------------------------------------------------------
// 08-05 Task 2 — the observability + remaining-surface pins: the iproute
// longest-match + verbs + smoltcp mirror, the ndb offset/OTRUNC/Eio/vers
// semantics, the netlog set→emit→blocked-reader-wakes cycle, and the
// generic ctl verbs' contract error strings on tcp.
// ---------------------------------------------------------------------------

/// A chan for a top-level /net file (the QTOPDIR-child qid shape).
fn top_file_chan(tag: u64) -> crate::dat::Chan {
    let q = crate::devip::qid(0, 0, tag);
    crate::dat::Chan::new(
        crate::dat::DEV_IP,
        crate::dat::Qid { path: q, vers: 0, typ: crate::dat::QTFILE },
        crate::dat::OREAD,
    )
}

/// The generic ctl verbs land on tcp with the contract's error strings
/// (contract §4 — the acceptance is "handled on EVERY proto's ctl"; the
/// pin exercises tcp — icmp/udp/ipifc share the same dispatch by
/// construction).
#[test]
fn tcp_generic_verbs_land_with_contract_errors() {
    let _serial = fresh_scripted_stack();
    let (id, _g) = alloc_tcp_conv();
    // maxfragsize (the source-only verb): bare → the needs-size error;
    // with a value → recorded on the conv.
    assert_eq!(
        proto::ctl_write(Proto::Tcp, id, b"maxfragsize").err().expect("pin (gen): the error"),
        "maxfragsize needs size"
    );
    proto::ctl_write(Proto::Tcp, id, b"maxfragsize 1400").expect("pin (gen): maxfragsize lands");
    {
        let node = crate::mcslock::LockNode::new();
        let t = crate::devip::conv::TABLES.lock(&node);
        assert_eq!(t.table(Proto::Tcp)[id].maxfragsize, 1400, "pin (gen): maxfragsize recorded");
    }
    // addmulti/remmulti's contract strings.
    assert_eq!(
        proto::ctl_write(Proto::Tcp, id, b"addmulti").err().expect("pin (gen): the error"),
        "addmulti needs interface address"
    );
    assert_eq!(
        proto::ctl_write(Proto::Tcp, id, b"addmulti 127.0.0.1").err().expect("pin (gen): the error"),
        "addmulti for a non multicast address",
        "pin (gen): a non-multicast raddr rejects"
    );
    assert_eq!(
        proto::ctl_write(Proto::Tcp, id, b"remmulti").err().expect("pin (gen): the error"),
        "remmulti needs interface address"
    );
    // ignoreadvice records.
    proto::ctl_write(Proto::Tcp, id, b"ignoreadvice").expect("pin (gen): ignoreadvice");
    {
        let node = crate::mcslock::LockNode::new();
        let t = crate::devip::conv::TABLES.lock(&node);
        assert!(t.table(Proto::Tcp)[id].ignoreadvice, "pin (gen): ignoreadvice recorded");
    }
}

/// The iproute forest: longest-match lookup, the verb semantics, the
/// contract line format, and the default route's smoltcp mirror.
#[test]
fn iproute_longest_match_verbs_and_smoltcp_mirror() {
    let _serial = fresh_scripted_stack();
    set_guest_addr();
    let route_ctl = |buf: &[u8]| crate::devip::route::ctl(buf);
    // Clean slate (the table is process-global; pins serialize).
    route_ctl(b"flush").expect("pin (route): flush");
    // A /8 and a /16 under it: the /16 wins inside it, the /8 covers the
    // rest, nothing matches outside.
    route_ctl(b"add 10.0.0.0 255.0.0.0 10.0.0.9").expect("pin (route): add /8");
    route_ctl(b"add 10.1.0.0 255.255.0.0 10.0.0.10").expect("pin (route): add /16");
    let r = crate::devip::route::lookup(&mapped(Ipv4Address::new(10, 1, 2, 3))).expect("pin (route): the /16 matches");
    assert_eq!(r.nexthop, mapped(Ipv4Address::new(10, 0, 0, 10)), "pin (route): longest-match picks the /16");
    let r = crate::devip::route::lookup(&mapped(Ipv4Address::new(10, 2, 3, 4))).expect("pin (route): the /8 matches");
    assert_eq!(r.nexthop, mapped(Ipv4Address::new(10, 0, 0, 9)), "pin (route): the /8 covers the rest");
    assert!(
        crate::devip::route::lookup(&mapped(Ipv4Address::new(11, 0, 0, 1))).is_none(),
        "pin (route): no match outside the table"
    );
    // The tag verb binds to subsequent adds.
    route_ctl(b"tag test").expect("pin (route): tag");
    route_ctl(b"add 192.168.0.0 255.255.0.0 10.0.0.11").expect("pin (route): tagged add");
    let r = crate::devip::route::lookup(&mapped(Ipv4Address::new(192, 168, 1, 1))).expect("pin (route): the tagged route");
    assert_eq!(&r.tag, b"test", "pin (route): the tag bound");
    // The read content is the contract's line format ("%-15I %-4M %-15I
    // %4.4s %4.4s %3s\n" — target mask nexthop flags tag ifc).
    let text = std::string::String::from_utf8(crate::devip::route::content()).expect("pin (route): utf8");
    assert!(text.contains("10.0.0.0"), "pin (route): the target column");
    assert!(text.contains("255.0.0.0"), "pin (route): the mask column");
    assert!(text.contains("10.0.0.9"), "pin (route): the nexthop column");
    assert!(text.contains("test"), "pin (route): the tag column");
    // remove: the /16 falls away, the /8 covers.
    route_ctl(b"remove 10.1.0.0 255.255.0.0").expect("pin (route): remove");
    let r = crate::devip::route::lookup(&mapped(Ipv4Address::new(10, 1, 2, 3))).expect("pin (route): falls through");
    assert_eq!(r.nexthop, mapped(Ipv4Address::new(10, 0, 0, 9)), "pin (route): the /8 covers after the remove");
    // The default route mirrors into smoltcp's Routes (and out on remove).
    route_ctl(b"add 0.0.0.0 0.0.0.0 10.0.0.2").expect("pin (route): add default");
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (route): stack installed");
        let def = core.iface.routes().get_default_ipv4_route();
        assert!(def.is_some(), "pin (route): the default route mirrored into smoltcp's Routes");
    }
    route_ctl(b"remove 0.0.0.0 0.0.0.0").expect("pin (route): remove default");
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (route): stack installed");
        assert!(
            core.iface.routes().get_default_ipv4_route().is_none(),
            "pin (route): the remove mirrored out of smoltcp's Routes"
        );
    }
    // flush empties the table.
    route_ctl(b"flush").expect("pin (route): flush");
    assert!(crate::devip::route::content().is_empty(), "pin (route): flush empties");
    // The verb error strings (contract §2/§11).
    assert_eq!(route_ctl(b"frobnicate").err().expect("pin (route): the error"), "bad arg in system call");
    assert_eq!(route_ctl(b"add 10.0.0.0 255.0.0.0").err().expect("pin (route): the error"), "bad arg in system call");
}

/// The ndb semantics (contract §2): append-at-offset, OWRITE|OTRUNC zeroes
/// first, Eio past end (`off > strlen` or `off+n >= 1024`), qid.vers bumps
/// on every write.
#[test]
fn ndb_offset_otrunc_eio_and_vers_bumps() {
    let _serial = fresh_scripted_stack();
    let base = top_file_chan(crate::devip::QNDB);
    // OWRITE|OTRUNC open zeroes the buffer first (the era's single-user
    // collapse dropped the donor's iseve gate).
    let mut c = crate::devip::open(&base, crate::dat::OWRITE | crate::dat::OTRUNC).expect("pin (ndb): open");
    let stat_vers = |c: &crate::dat::Chan| {
        let mut sb = [0u8; 128];
        let n = crate::devip::stat(c, &mut sb).expect("pin (ndb): stat") as usize;
        assert!(n >= 11, "pin (ndb): a full stat record");
        u32::from_le_bytes([sb[7], sb[8], sb[9], sb[10]])
    };
    let v0 = stat_vers(&c);
    crate::devip::write(&mut c, b"hello", 0).expect("pin (ndb): write at 0");
    let v1 = stat_vers(&c);
    assert!(v1 > v0, "pin (ndb): vers bumped on the write");
    let mut buf = [0u8; 16];
    let n = crate::devip::read(&mut c, &mut buf, 0).expect("pin (ndb): read back") as usize;
    assert_eq!(&buf[..n], b"hello", "pin (ndb): the content read back");
    // Append at the CURRENT end (off == strlen is legal; off > strlen is Eio).
    crate::devip::write(&mut c, b" world", 5).expect("pin (ndb): append at end");
    let n = crate::devip::read(&mut c, &mut buf, 0).expect("pin (ndb): re-read") as usize;
    assert_eq!(&buf[..n], b"hello world", "pin (ndb): the append landed at the offset");
    assert_eq!(
        crate::devip::write(&mut c, b"x", 12).err().expect("pin (ndb): the error"),
        "i/o error",
        "pin (ndb): off > strlen → Eio"
    );
    // off+n >= 1024 → Eio (the donor's 1024-byte Fs.ndb bound).
    assert_eq!(
        crate::devip::write(&mut c, &[0u8; 8], 1020).err().expect("pin (ndb): the error"),
        "i/o error",
        "pin (ndb): off+n >= 1024 → Eio"
    );
    // A fresh OWRITE|OTRUNC open zeroes again (the content is gone).
    let mut c2 = crate::devip::open(&base, crate::dat::OWRITE | crate::dat::OTRUNC).expect("pin (ndb): reopen otrunc");
    let n = crate::devip::read(&mut c2, &mut buf, 0).expect("pin (ndb): read after otrunc") as usize;
    assert_eq!(n, 0, "pin (ndb): OWRITE|OTRUNC zeroed the buffer");
}

/// The netlog cycle (D-07 + contract §2's `log`): set arms the flag, the
/// gated emit lands the line, and the reader — BLOCKED on the empty queue
/// first — wakes and pops it.
#[test]
fn netlog_set_emit_wakes_blocked_reader() {
    let _serial = fresh_scripted_stack();
    // A clean slate: clear the flags and drain anything queued.
    crate::devip::netlog::ctl(b"clear tcp udp icmp ip").expect("pin (netlog): clear");
    let _ = crate::devip::netlog::drain_lines();
    // The verb error strings (contract §2): bare set/clear/only → Ebadnetctl.
    assert_eq!(
        crate::devip::netlog::ctl(b"set").err().expect("pin (netlog): the error"),
        "too few arguments for netlog control message"
    );
    // Gated: with the flag clear the emit is dropped.
    crate::devip::netlog::emit(crate::devip::netlog::LOG_TCP, b"tcp: dropped note".to_vec());
    assert!(crate::devip::netlog::drain_lines().is_empty(), "pin (netlog): gated when clear");
    // Set the flag; the reader parks on the empty queue; the emit lands
    // from INSIDE the park (the on-block seam — the park is recorded).
    crate::devip::netlog::ctl(b"set tcp").expect("pin (netlog): set tcp");
    let before = block_count();
    set_on_block(|| {
        crate::devip::netlog::emit(crate::devip::netlog::LOG_TCP, b"tcp: test note".to_vec());
    });
    let mut buf = [0u8; 128];
    let n = crate::devip::netlog::read(&mut buf).expect("pin (netlog): the read returns");
    assert_eq!(&buf[..n as usize], b"tcp: test note", "pin (netlog): the line landed verbatim");
    assert!(block_count() > before, "pin (netlog): the read BLOCKED before the emit landed");
    assert!(
        take_wake_log().contains(&STUB_PID),
        "pin (netlog): the emit woke the blocked reader (the wake-after-drop drain)"
    );
    // only <addr> records the restriction (the ether era's per-packet gate).
    crate::devip::netlog::ctl(b"only 10.0.0.1").expect("pin (netlog): only");
}

// ---------------------------------------------------------------------------
// The ETHER leg's pins (08-06): the EtherDev arrival-order merge across
// the three netif conv queues (0x800/0x806/0x86dd), the /net/arp snoop
// feeding OK/WAIT entries BOTH directions, the gratuitous-ARP emission on
// add, and the address-class demux (contract §12.4).  The scripted ether
// device (ScriptedEtherDev) is Medium::Ethernet: whole 14-byte-header
// frames; TX frames are captured; the snoop feed is live through the
// real receive()/transmit() call sites.
// ---------------------------------------------------------------------------

/// The scripted ether MACs (guest + gateway).
const GUEST_MAC: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
const GW_MAC: [u8; 6] = [0x52, 0x55, 0x0a, 0x00, 0x02, 0x02];
/// The scripted guest/gateway v4 addresses (the slirp shape).
const GW4: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);
const GUEST4: Ipv4Address = Ipv4Address::new(10, 0, 2, 15);

/// Craft a WHOLE ARP frame (14-byte ethernet header + the 28-byte ARP
/// payload) with smoltcp's wire codecs.
fn arp_eth_frame(
    op: smoltcp::wire::ArpOperation,
    sha: [u8; 6],
    spa: Ipv4Address,
    tpa: Ipv4Address,
) -> Vec<u8> {
    use smoltcp::wire::{
        ArpPacket, ArpRepr, EthernetAddress, EthernetFrame, EthernetProtocol, EthernetRepr,
    };
    let repr = ArpRepr::EthernetIpv4 {
        operation: op,
        source_hardware_addr: EthernetAddress(sha),
        source_protocol_addr: spa,
        target_hardware_addr: EthernetAddress([0; 6]),
        target_protocol_addr: tpa,
    };
    let mut frame = alloc_vec(14 + 28);
    let mut ef = EthernetFrame::new_unchecked(&mut frame[..]);
    EthernetRepr {
        src_addr: EthernetAddress(sha),
        dst_addr: EthernetAddress([0xff; 6]),
        ethertype: EthernetProtocol::Arp,
    }
    .emit(&mut ef);
    let mut pkt = ArpPacket::new_unchecked(ef.payload_mut());
    repr.emit(&mut pkt);
    frame
}

/// Craft a minimal whole-frame IPv4 packet (the 0x800 lane) — `tag` is
/// the frame's distinguishing last byte (the merge pin's identity).
fn ipv4_eth_frame(tag: u8) -> Vec<u8> {
    let mut bytes = alloc_vec(14 + 20);
    {
        let mut ef = smoltcp::wire::EthernetFrame::new_unchecked(&mut bytes[..]);
        smoltcp::wire::EthernetRepr {
            src_addr: smoltcp::wire::EthernetAddress(GUEST_MAC),
            dst_addr: smoltcp::wire::EthernetAddress(GW_MAC),
            ethertype: smoltcp::wire::EthernetProtocol::Ipv4,
        }
        .emit(&mut ef);
    }
    {
        let mut ip = Ipv4Packet::new_unchecked(&mut bytes[14..]);
        Ipv4Repr {
            src_addr: GUEST4,
            dst_addr: GW4,
            next_header: IpProtocol::Tcp,
            payload_len: 0,
            hop_limit: 64,
        }
        .emit(&mut ip, &ChecksumCapabilities::default());
    }
    bytes[14 + 19] = tag; // the IP header's last byte carries the tag
    bytes
}

/// Craft a minimal whole-frame IPv6 packet (the 0x86dd lane; next_header
/// TCP — NOT ICMPv6, so the NDP snoop skips it).
fn ipv6_eth_frame(tag: u8) -> Vec<u8> {
    let mut bytes = alloc_vec(14 + 40);
    {
        let mut ef = smoltcp::wire::EthernetFrame::new_unchecked(&mut bytes[..]);
        smoltcp::wire::EthernetRepr {
            src_addr: smoltcp::wire::EthernetAddress(GUEST_MAC),
            dst_addr: smoltcp::wire::EthernetAddress(GW_MAC),
            ethertype: smoltcp::wire::EthernetProtocol::Ipv6,
        }
        .emit(&mut ef);
    }
    // Version 6 (the top nibble of byte 0) + the tag in the last byte.
    bytes[14] = 0x60 | (tag >> 4);
    bytes[14 + 39] = tag;
    bytes
}

/// Pin 1: the EtherDev RX merge pops the three conv queues in ARRIVAL
/// order (the global ingress seq stamps), not lane order.
#[test]
fn etherdev_merges_the_three_conv_queues_in_arrival_order() {
    use smoltcp::phy::{Device, RxToken};
    let _serial = fresh_scripted_stack();
    crate::devip::arp::test_flush();
    let mut dev = crate::devip::phyadapt::ScriptedEtherDev::new(GUEST_MAC);
    // Out-of-order arrival across the three lanes: seq 3 on the IPv4
    // lane, seq 1 on the IPv6 lane, seq 2 on the ARP lane.  (The ARP
    // lane's frame carries a zeroed payload — unparseable as ARP, so the
    // snoop skips it and the merge pin stays isolated from the table.)
    let f_v6 = ipv6_eth_frame(1);
    let f_arp = {
        let mut f = alloc_vec(14 + 28);
        f[12] = 0x08;
        f[13] = 0x06;
        f
    };
    let f_v4 = ipv4_eth_frame(3);
    dev.push_rx(3, f_v4.clone());
    dev.push_rx(1, f_v6.clone());
    dev.push_rx(2, f_arp.clone());
    for expected in [&f_v6, &f_arp, &f_v4] {
        let (rx, _tx) = dev.receive(smoltcp::time::Instant::ZERO).expect("pin (merge): a frame");
        rx.consume(|got| {
            assert_eq!(got, expected.as_slice(), "pin (merge): arrival order, not lane order");
        });
    }
    assert!(
        dev.receive(smoltcp::time::Instant::ZERO).is_none(),
        "pin (merge): the queues drained"
    );
}

/// Pin 2: the /net/arp snoop feeds OK/WAIT entries on scripted frames in
/// BOTH directions — an RX reply records (ip→mac, OK); a TX solicitation
/// records (tpa, WAIT); the TX gratuitous announcement records OK; the
/// WAIT completes to OK when the RX reply arrives.  Plus the file
/// surface's 90-byte line + the offset%90 gate.
#[test]
fn arp_snoop_feeds_ok_and_wait_entries_both_directions() {
    use smoltcp::phy::{Device, RxToken, TxToken};
    let _serial = fresh_scripted_stack();
    crate::devip::arp::test_flush();
    let mut dev = crate::devip::phyadapt::ScriptedEtherDev::new(GUEST_MAC);

    // RX: the gateway's ARP reply (gw is GW_MAC) — an OK entry.
    let reply = arp_eth_frame(smoltcp::wire::ArpOperation::Reply, GW_MAC, GW4, GUEST4);
    {
        dev.push_rx(1, reply);
        let (rx, _tx) = dev.receive(smoltcp::time::Instant::ZERO).expect("pin (arp): the reply");
        rx.consume(|_| {});
    }
    let gw16 = crate::devip::v4prefix_of(&GW4.octets());
    assert_eq!(
        crate::devip::arp::test_lookup(&gw16),
        Some((GW_MAC, true)),
        "pin (arp): the RX reply fed (gw→mac, OK)"
    );

    // TX: our own solicitation (who-has 10.0.2.3 tell guest) — a WAIT
    // entry keyed by the TARGET.
    let server = Ipv4Address::new(10, 0, 2, 3);
    let solicit = arp_eth_frame(smoltcp::wire::ArpOperation::Request, GUEST_MAC, GUEST4, server);
    {
        let tx = dev.transmit(smoltcp::time::Instant::ZERO).expect("pin (arp): a tx token");
        tx.consume(solicit.len(), |buf| buf.copy_from_slice(&solicit));
    }
    let srv16 = crate::devip::v4prefix_of(&server.octets());
    assert_eq!(
        crate::devip::arp::test_lookup(&srv16),
        Some(([0; 6], false)),
        "pin (arp): the TX solicitation fed (target, WAIT)"
    );

    // RX: the server's reply completes the WAIT → OK with its mac.
    let answer = arp_eth_frame(smoltcp::wire::ArpOperation::Reply, GW_MAC, server, GUEST4);
    {
        dev.push_rx(1, answer);
        let (rx, _tx) = dev.receive(smoltcp::time::Instant::ZERO).expect("pin (arp): the reply");
        rx.consume(|_| {});
    }
    assert_eq!(
        crate::devip::arp::test_lookup(&srv16),
        Some((GW_MAC, true)),
        "pin (arp): the reply completed WAIT → OK"
    );

    // The gratuitous TX (spa == tpa) records the announcer as OK (not
    // WAIT — an announcement is not a solicitation).
    let garp = crate::devip::phyadapt::craft_gratuitous_arp(GUEST_MAC, GUEST4.octets());
    {
        let tx = dev.transmit(smoltcp::time::Instant::ZERO).expect("pin (arp): tx for the garp");
        tx.consume(garp.len(), |buf| buf.copy_from_slice(&garp));
    }
    let g16 = crate::devip::v4prefix_of(&GUEST4.octets());
    assert_eq!(
        crate::devip::arp::test_lookup(&g16),
        Some((GUEST_MAC, true)),
        "pin (arp): the gratuitous TX announcement fed (self, OK)"
    );

    // The file surface: the 90-byte line shape with the real "ether"
    // medium field, and the offset%90 gate (arp.c's arpread).
    let mut chan = crate::dat::Chan::new(
        crate::dat::DEV_IP,
        crate::dat::Qid { path: crate::devip::qid(0, 0, crate::devip::QARP), vers: 0, typ: 0 },
        0,
    );
    let mut one = [0u8; 4];
    let n = crate::devip::arp::read(&mut chan, &mut one, 1).expect("pin (arp): the read");
    assert_eq!(n, 0, "pin (arp): offset % 90 != 0 reads 0");
    let mut buf = alloc_vec(90 * 4);
    let n = crate::devip::arp::read(&mut chan, &mut buf, 0).expect("pin (arp): the aligned read")
        as usize;
    assert_eq!(n % 90, 0, "pin (arp): whole lines only");
    assert!(n >= 3 * 90, "pin (arp): the three fed entries print");
    // The gateway's line: "ether  OK       10.0.2.2 ... 52:55:0a:00:02:02".
    let text = std::string::String::from_utf8_lossy(&buf[..n]);
    assert!(text.contains("ether"), "pin (arp): the medium field names ether");
    assert!(text.contains("10.0.2.2"), "pin (arp): the gateway line");
    assert!(text.contains("52:55:0a:00:02:02"), "pin (arp): the gateway mac");
}

/// Pin 3: the gratuitous-ARP emission content (the donor's
/// areg=sendgarp): the crafted frame is a broadcast who-has <ip> tell
/// <ip> REQUEST from the guest MAC — and the seam-dispatch emission (the
/// ipifc add arm's exact call shape) lands it in the scripted device's
/// TX capture.
#[test]
fn gratuitous_arp_emission_content_on_add() {
    let _serial = fresh_scripted_stack();
    crate::devip::arp::test_flush();
    crate::devip::stack::install_test_ether(StackCore::new_scripted_ether(
        crate::devip::phyadapt::ScriptedEtherDev::new(GUEST_MAC),
    ));
    // The ipifc add arm's exact call shape (the ether core's device gets
    // the sendgarp).
    {
        let mut g = stack::enter_stack();
        let core = g.core_mut(stack::CoreSel::Ether).expect("pin (garp): the ether core");
        core.dev.send_gratuitous_arp(GUEST4.octets());
    }
    let frame = {
        let g = stack::enter_stack();
        let core = g.core_ref(stack::CoreSel::Ether).expect("pin (garp): the ether core");
        let crate::devip::phyadapt::StackDev::ScriptedEther(dev) = &core.dev else {
            panic!("pin (garp): not the scripted ether device");
        };
        assert_eq!(dev.tx_log.len(), 1, "pin (garp): exactly one frame emitted");
        dev.tx_log[0].clone()
    };
    // Parse it with the wire codecs: broadcast, ethertype ARP, op
    // Request, sha = the guest MAC, spa == tpa == the guest address.
    let ef = smoltcp::wire::EthernetFrame::new_checked(&frame[..]).expect("pin (garp): eth parse");
    assert!(ef.dst_addr().is_broadcast(), "pin (garp): broadcast");
    assert_eq!(ef.src_addr().0, GUEST_MAC, "pin (garp): src is the guest MAC");
    assert_eq!(ef.ethertype(), smoltcp::wire::EthernetProtocol::Arp, "pin (garp): the 0x806 ethertype");
    let pkt = smoltcp::wire::ArpPacket::new_checked(ef.payload()).expect("pin (garp): arp parse");
    let repr = smoltcp::wire::ArpRepr::parse(&pkt).expect("pin (garp): the repr");
    let (operation, source_hardware_addr, source_protocol_addr, target_protocol_addr) = match repr {
        smoltcp::wire::ArpRepr::EthernetIpv4 {
            operation,
            source_hardware_addr,
            source_protocol_addr,
            target_protocol_addr,
            ..
        } => (operation, source_hardware_addr, source_protocol_addr, target_protocol_addr),
        _ => panic!("pin (garp): not EthernetIpv4"),
    };
    assert_eq!(operation, smoltcp::wire::ArpOperation::Request, "pin (garp): a who-has REQUEST");
    assert_eq!(source_hardware_addr.0, GUEST_MAC, "pin (garp): sha is the guest");
    assert_eq!(source_protocol_addr, GUEST4, "pin (garp): spa is the guest");
    assert_eq!(target_protocol_addr, GUEST4, "pin (garp): tpa == spa (tell-yourself)");
}

/// Pin 4: the address-class demux (contract §12.4): 127/8 and ::1 → the
/// loopback core; everything else → the ether core WHEN BOUND, else the
/// loopback core (the single-core fallback keeps the 08-04/08-05 gates'
/// shape).
#[test]
fn address_class_demux_loopback_vs_ether() {
    let _serial = fresh_scripted_stack();
    let lo = crate::devip::v4prefix_of(&[127, 1, 2, 3]);
    let other = crate::devip::v4prefix_of(&[10, 0, 2, 15]);
    let v6lo = {
        let mut a = [0u8; 16];
        a[15] = 1;
        a
    };
    let v6other = {
        let mut a = [0u8; 16];
        a[0] = 0x20;
        a[15] = 1;
        a
    };
    {
        // Loopback-only (the 08-04/08-05 era): everything demuxes to the
        // loopback core (the fallback — no ether bound).
        let g = stack::enter_stack();
        assert_eq!(g.demux(&lo), stack::CoreSel::Loopback, "pin (demux): 127/8 → loopback");
        assert_eq!(g.demux(&v6lo), stack::CoreSel::Loopback, "pin (demux): ::1 → loopback");
        assert_eq!(g.demux(&other), stack::CoreSel::Loopback, "pin (demux): fallback when no ether");
    }
    // Bind the ether core: the demux table's other arm goes live.
    crate::devip::stack::install_test_ether(StackCore::new_scripted_ether(
        crate::devip::phyadapt::ScriptedEtherDev::new(GUEST_MAC),
    ));
    {
        let g = stack::enter_stack();
        assert_eq!(g.demux(&lo), stack::CoreSel::Loopback, "pin (demux): 127/8 → loopback (ether bound)");
        assert_eq!(g.demux(&v6lo), stack::CoreSel::Loopback, "pin (demux): ::1 → loopback (ether bound)");
        assert_eq!(g.demux(&other), stack::CoreSel::Ether, "pin (demux): other v4 → ether");
        assert_eq!(g.demux(&v6other), stack::CoreSel::Ether, "pin (demux): other v6 → ether");
        // The wildcard (announce's noaddr) rides the ether core when bound.
        assert_eq!(g.demux(&[0; 16]), stack::CoreSel::Ether, "pin (demux): wildcard → ether when bound");
    }
}

/// Pin 5 (the 08-06 must_haves' backstop truth): ipifc add/remove at the
/// boundary values follows the contract doc's behavior with the exact
/// error strings — the 0.0.0.0/0 unnumbered DHCP preconfig ABSORBS (never
/// a smoltcp panic, P-F), a duplicate add is a no-op, an invalid mask
/// errors "bad ip address syntax", remove of a missing lifc errors
/// "devip: no such local address", and a real remove succeeds.
#[test]
fn ipifc_add_remove_boundary_values_follow_the_contract() {
    // A bare seam: NO pre-installed core — the bind below installs the
    // loopback core itself (fresh_scripted_stack's install would trip the
    // bind's already-bound guard).
    let _serial = SEAM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_hooks();
    set_ticks(0);
    take_wake_log();
    {
        let mut g = stack::enter_stack();
        g.core = None;
        g.ether = None;
    }
    crate::devip::arp::test_flush();
    use crate::devip::{conv, ipifc};
    let id = conv::conv_alloc(conv::Proto::Ipifc).expect("pin (bound): ipifc clone");
    ipifc::ctl(id, b"bind", &[b"loopback"]).expect("pin (bound): bind loopback");
    // The absorption (contract §12.6 / P-F): the unspecified add records
    // the DHCP-in-progress marker and NEVER reaches smoltcp's
    // update_ip_addrs (which would panic) — the write succeeds.  The
    // donor's addselfcache null-address side effect fires: ipforme's
    // accept-all flag sets (the DHCP era's accept-all state).
    ipifc::ctl(id, b"add", &[b"0.0.0.0", b"0.0.0.0"]).expect("pin (bound): the absorbed add");
    assert!(
        crate::devip::selftab_acceptall(),
        "pin (bound): the absorbed 0.0.0.0 set the accept-all flag (addselfcache's side effect)"
    );
    // A real address, then its DUPLICATE (the donor's already-a-local
    // no-op, ipifc.c:423-437).
    ipifc::ctl(id, b"add", &[b"127.0.0.1", b"255.0.0.0"]).expect("pin (bound): the real add");
    ipifc::ctl(id, b"add", &[b"127.0.0.1", b"255.0.0.0"]).expect("pin (bound): the duplicate no-op");
    // The invalid mask errors with the exact string.
    let e = ipifc::ctl(id, b"add", &[b"127.0.0.2", b"999.0.0.0"]).expect_err("pin (bound): invalid mask errors");
    assert_eq!(e, "bad ip address syntax", "pin (bound): the invalid-mask string");
    // remove of a missing lifc errors; a real remove succeeds.
    let e = ipifc::ctl(id, b"remove", &[b"10.9.9.9", b"255.255.255.255"])
        .expect_err("pin (bound): remove of a missing lifc errors");
    assert_eq!(e, "devip: no such local address", "pin (bound): the not-found string");
    ipifc::ctl(id, b"remove", &[b"127.0.0.1", b"255.0.0.0"]).expect("pin (bound): the real remove");
    // Clean teardown (the FS/static seam state survives across pins):
    // drop the absorbed marker (accept-all clears) + unbind + close.
    ipifc::ctl(id, b"remove", &[b"0.0.0.0", b"0.0.0.0"]).expect("pin (bound): drop the marker");
    assert!(!crate::devip::selftab_acceptall(), "pin (bound): the marker's remove clears accept-all");
    ipifc::ctl(id, b"unbind", &[]).expect("pin (bound): unbind");
    conv::conv_close(conv::Proto::Ipifc, id);
}

/// The P-F absorption's v4 extensions (CR-04): `add 224.0.0.1` (v4
/// multicast) and `add 255.255.255.255` (the limited broadcast) must
/// absorb record-only — smoltcp's check_ip_addrs PANICS on any address
/// that is neither unicast nor unspecified (224/4 fails the unicast arm;
/// the all-ones address is is_broadcast), so pushing either turns an
/// unprivileged ctl write into a kernel panic.  The lifc records (and
/// removes) cleanly; the iface's address list never moves.
#[test]
fn ipifc_add_v4_multicast_and_broadcast_absorb_without_panic() {
    // A bare seam (the bind installs the core itself) — the boundary
    // pin's shape.
    let _serial = SEAM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_hooks();
    set_ticks(0);
    take_wake_log();
    {
        let mut g = stack::enter_stack();
        g.core = None;
        g.ether = None;
    }
    crate::devip::arp::test_flush();
    use crate::devip::{conv, ipifc};
    let id = conv::conv_alloc(conv::Proto::Ipifc).expect("pin (mcast4): ipifc clone");
    ipifc::ctl(id, b"bind", &[b"loopback"]).expect("pin (mcast4): bind loopback");

    ipifc::ctl(id, b"add", &[b"224.0.0.1", b"255.0.0.0"]).expect("pin (mcast4): the v4-multicast add absorbs");
    ipifc::ctl(id, b"add", &[b"255.255.255.255", b"255.255.255.255"])
        .expect("pin (mcast4): the limited-broadcast add absorbs");
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (mcast4): the loopback core");
        assert!(
            core.iface.ip_addrs().is_empty(),
            "pin (mcast4): neither absorption reaches smoltcp (P-F)"
        );
    }
    // The records are real lifcs (removable — recorded, not dropped).
    ipifc::ctl(id, b"remove", &[b"224.0.0.1", b"255.0.0.0"]).expect("pin (mcast4): the mcast record removes");
    ipifc::ctl(id, b"remove", &[b"255.255.255.255", b"255.255.255.255"])
        .expect("pin (mcast4): the broadcast record removes");

    ipifc::ctl(id, b"unbind", &[]).expect("pin (mcast4): unbind");
    conv::conv_close(conv::Proto::Ipifc, id);
}

/// The unbind orphan sweep (CR-05): convs hold sockets/pools in the seam
/// core independent of the BINDING conv's lifetime.  Dropping the core
/// under them leaves stale SocketHandles; a later re-bind's fresh
/// SocketSet reuses indices from zero, turning the orphaned conv's next
/// op into a smoltcp panic or a silent alias of another conversation's
/// socket.  The sweep orphans every conv whose sel names the dying core
/// BEFORE the drop: references cleared, hangup set, waiters woken.
#[test]
fn ipifc_unbind_orphans_convs_on_the_dying_core() {
    // A bare seam; bind loopback, then swap in the scripted capture
    // device (the v6rt pin's shape — the scripted peer answers the tcp
    // handshake).
    let _serial = SEAM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_hooks();
    set_ticks(0);
    take_wake_log();
    {
        let mut g = stack::enter_stack();
        g.core = None;
        g.ether = None;
    }
    crate::devip::arp::test_flush();
    use crate::devip::{conv, ipifc};
    let ifc = conv::conv_alloc(conv::Proto::Ipifc).expect("pin (orphan): ipifc clone");
    ipifc::ctl(ifc, b"bind", &[b"loopback"]).expect("pin (orphan): bind loopback");
    {
        let mut g = stack::enter_stack();
        g.core = Some(StackCore::new_scripted(ScriptedDev::new()));
    }
    set_guest_addr();

    // A live tcp conv (the scripted peer completes the handshake) and a
    // connected udp conv — both ride the loopback core (no ether bound).
    let (t, _gt) = alloc_tcp_conv();
    scripted_connect(t, 1300);
    assert!(tcp_sock_state(t).is_some(), "pin (orphan): the tcp conv holds a socket");
    let (u, _gu) = alloc_udp_conv();
    proto::ctl_write(Proto::Udp, u, b"connect 10.0.0.2!7000").expect("pin (orphan): udp connect");

    // The unbind (ridden by the BINDING conv, not theirs) orphans both.
    ipifc::ctl(ifc, b"unbind", &[]).expect("pin (orphan): unbind");
    assert_eq!(tcp_sock_state(t), None, "pin (orphan): the tcp conv's socket reference is cleared");
    assert!(tcp_conv_snapshot(t).5, "pin (orphan): the tcp conv carries the hangup");
    let (hu, _) = udp_conv_hangup_err(u);
    assert!(hu.is_some(), "pin (orphan): the udp conv carries the hangup");
    let e = proto::tcp_read_data(t, &mut [0u8; 16]).expect_err("pin (orphan): the orphaned tcp read errors");
    assert_eq!(e, "i/o on hungup channel", "pin (orphan): the hangup error, not a socket touch");
    assert!(proto::udp_write_data(u, b"x").is_err(), "pin (orphan): the orphaned udp write errors");

    // The poison window: a RE-bind installs a fresh core whose SocketSet
    // reuses indices from zero — touching the orphaned conv must error,
    // never panic or alias a successor socket.
    ipifc::ctl(ifc, b"bind", &[b"loopback"]).expect("pin (orphan): re-bind");
    let e = proto::tcp_read_data(t, &mut [0u8; 16]).expect_err("pin (orphan): the post-rebind read errors");
    assert_eq!(e, "i/o on hungup channel", "pin (orphan): no stale-handle touch after re-bind");
    assert!(proto::udp_write_data(u, b"x").is_err(), "pin (orphan): the post-rebind udp write errors");

    // Teardown: drop the second binding (the orphans hold no seam
    // references, so this unbind's sweep is a no-op) and close the ifc.
    ipifc::ctl(ifc, b"unbind", &[]).expect("pin (orphan): teardown unbind");
    conv::conv_close(conv::Proto::Ipifc, ifc);
}

// ---------------------------------------------------------------------------
// 08-07 Task 1 — the NETIP-05 DHCP pins: the P-F absorption's file-level
// shape (zero smoltcp addresses, the netlog note — D-07 observability) and
// the broadcast TX shape THROUGH the devip file surface (the 08-02 OQ-1
// pin (h)'s raw-seam verdict extended to the conv/ctl/data path the real
// ipconfig drives).
// ---------------------------------------------------------------------------

/// The absorption pin (contract §12.6 / P-F, the plan's Task-1 pin 2a):
/// an `add 0.0.0.0 0.0.0.0` ctl write on a bound ifc SUCCEEDS, leaves ZERO
/// configured addresses on the iface (smoltcp's update_ip_addrs is never
/// called with it — it would PANIC on the non-unicast address), and the
/// absorption is note-worthy in netlog (D-07: deliberate but
/// wire-invisible events are observable).
#[test]
fn ipifc_dhcp_preconfig_absorbs_with_zero_iface_addresses_and_a_netlog_note() {
    // A bare seam (the bind installs the core itself) — the boundary
    // pin's shape.
    let _serial = SEAM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_hooks();
    set_ticks(0);
    take_wake_log();
    {
        let mut g = stack::enter_stack();
        g.core = None;
        g.ether = None;
    }
    crate::devip::arp::test_flush();
    use crate::devip::{conv, ipifc, netlog};
    // The note is flag-gated (netlog's emit drops when the flag is off):
    // enable `ip` first; drain any stale lines so the assertion reads
    // only THIS pin's output.
    netlog::ctl(b"set ip").expect("pin (absorb): set ip");
    let _ = netlog::drain_lines();

    let id = conv::conv_alloc(conv::Proto::Ipifc).expect("pin (absorb): ipifc clone");
    ipifc::ctl(id, b"bind", &[b"loopback"]).expect("pin (absorb): bind loopback");
    ipifc::ctl(id, b"add", &[b"0.0.0.0", b"0.0.0.0"]).expect("pin (absorb): the absorbed add");

    // Zero configured addresses on the iface (the marker never reached
    // smoltcp).
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (absorb): the loopback core");
        assert!(
            core.iface.ip_addrs().is_empty(),
            "pin (absorb): the absorbed add leaves zero smoltcp addresses (P-F)"
        );
    }
    assert!(
        crate::devip::selftab_acceptall(),
        "pin (absorb): the addselfcache null-address side effect (accept-all) set"
    );
    let lines = netlog::drain_lines();
    assert!(
        lines.iter().any(|l| l.windows(b"absorbed".len()).any(|w| w == b"absorbed")),
        "pin (absorb): the netlog note records the absorption (D-07); got {lines:?}"
    );

    // Teardown: drop the marker + unbind + close; clear the ip flag.
    ipifc::ctl(id, b"remove", &[b"0.0.0.0", b"0.0.0.0"]).expect("pin (absorb): drop the marker");
    ipifc::ctl(id, b"unbind", &[]).expect("pin (absorb): unbind");
    conv::conv_close(conv::Proto::Ipifc, id);
    netlog::ctl(b"clear ip").expect("pin (absorb): clear ip");
}

/// The broadcast-TX pin (the plan's Task-1 pin 2b): a Bootp-sized
/// datagram written to a port-68 headers-mode conv in the absorbed
/// DHCP-in-progress state leaves the adapter with **src 0.0.0.0** +
/// **dst 255.255.255.255:67** (udp 68 → 67, payload intact) — the OQ-1
/// PASS-path verdict (contract §12.6) driven through the devip file
/// surface the real ipconfig exercises (`announce *!68` → `headers` →
/// the full-size Bootp write, main.c:1082-1170).  The Offer's return
/// leg (broadcast dst to port 68, still no configured address) reads
/// back the 52-byte Udphdr prefix + the payload.
#[test]
fn dhcp_broadcast_tx_shape_through_the_devip_file_surface() {
    // Bare seam; the loopback bind installs the core, then the scripted
    // device REPLACES it (the pin asserts the IP-level wire shape — the
    // medium (loopback-vs-ether) never alters src/dst at that layer; the
    // ether adapter's construction needs the live netif machinery the
    // host lacks).
    let _serial = SEAM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_hooks();
    set_ticks(0);
    take_wake_log();
    {
        let mut g = stack::enter_stack();
        g.core = None;
        g.ether = None;
    }
    crate::devip::arp::test_flush();
    use crate::devip::{conv, ipifc, proto};

    let ifc_id = conv::conv_alloc(conv::Proto::Ipifc).expect("pin (dora): ipifc clone");
    ipifc::ctl(ifc_id, b"bind", &[b"loopback"]).expect("pin (dora): bind loopback");
    {
        let mut g = stack::enter_stack();
        g.core = Some(StackCore::new_scripted(ScriptedDev::new()));
    }
    // The DHCP preconfig (main.c:926's `add %I %I` of IPnoaddr):
    // absorbed devip-side, accept-all set.
    ipifc::ctl(ifc_id, b"add", &[b"0.0.0.0", b"0.0.0.0"]).expect("pin (dora): the absorbed add");
    assert!(crate::devip::selftab_acceptall(), "pin (dora): the DHCP-in-progress marker");

    // The udp conv exactly as ipconfig's openlisten builds it
    // (main.c:1381-1411): announce udp!*!68 (libc writes `announce
    // *!68`), then `headers`.
    let (id, _guard) = alloc_udp_conv();
    proto::ctl_write(conv::Proto::Udp, id, b"announce *!68").expect("pin (dora): announce *!68");
    proto::ctl_write(conv::Proto::Udp, id, b"headers").expect("pin (dora): headers");
    let (_, hdrs, laddr, lport, _, _, _, _, _) = udp_conv_snapshot(id);
    assert_eq!(lport, 68, "pin (dora): announced port 68");
    assert_eq!(hdrs, 7, "pin (dora): headers=7 recorded");
    assert_eq!(laddr, [0; 16], "pin (dora): the wildcard announce's laddr is unspecified");

    // The Bootp-shaped headers-mode write: the 52-byte Udphdr prefix
    // (raddr = broadcast, laddr = zeros — dhcpsend's memset shape) + the
    // Bootp fixed part + magic cookie + the marker tail.
    let mut wbuf = alloc_vec(52);
    wbuf[0..16].copy_from_slice(&mapped(Ipv4Address::BROADCAST)); // raddr
    // laddr[16..32] stays zeros (the memset 0 of dhcpsend's Bootp.udphdr)
    wbuf[48..50].copy_from_slice(&67u16.to_be_bytes()); // rport
    wbuf[50..52].copy_from_slice(&68u16.to_be_bytes()); // lport
    let payload = bootp_shaped_payload(DHCP_MARKER);
    let n_payload = payload.len();
    wbuf.extend_from_slice(&payload);
    let n = proto::udp_write_data(id, &wbuf).expect("pin (dora): the write succeeds");
    assert_eq!(n as usize, wbuf.len(), "pin (dora): the write count");

    // The adapter's capture: exactly one frame, src 0.0.0.0 ON THE WIRE,
    // dst 255.255.255.255, udp 68 → 67, payload intact.
    assert_eq!(tx_count(), 1, "pin (dora): the Discover left the adapter (never the silent drop)");
    let (src, dst, sport, dport, got) = {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (dora): the core");
        parse_udp4(&core.dev_scripted().tx_log[0])
    };
    assert_eq!(src, Ipv4Address::UNSPECIFIED, "pin (dora): OQ-1's src 0.0.0.0 through the file surface");
    assert_eq!(dst, Ipv4Address::BROADCAST, "pin (dora): broadcast dst");
    assert_eq!((sport, dport), (68, 67), "pin (dora): the bootp ports");
    assert_eq!(got.len(), n_payload, "pin (dora): the full Bootp payload");
    assert!(got.ends_with(DHCP_MARKER), "pin (dora): payload integrity");

    // The Offer's return leg: broadcast dst to port 68 with STILL no
    // configured address — accepted (udp accepts(): port match +
    // broadcast dst); the headers-mode read synthesizes the 52-byte
    // Udphdr prefix (raddr = the server, rport 67, lport 68).
    let offer_payload = bootp_shaped_payload(OFFER_MARKER);
    push_rx(udp4_frame(DHCP_SERVER, Ipv4Address::BROADCAST, 67, 68, &offer_payload));
    let mut rbuf = alloc_vec(4096);
    let n = proto::udp_read_data(id, &mut rbuf).expect("pin (dora): the Offer reads") as usize;
    assert_eq!(n, 52 + offer_payload.len(), "pin (dora): the prefixed datagram");
    assert_eq!(&rbuf[0..16], &mapped(DHCP_SERVER), "pin (dora): raddr = the server");
    assert_eq!(&rbuf[48..50], &67u16.to_be_bytes(), "pin (dora): rport big-endian");
    assert_eq!(&rbuf[50..52], &68u16.to_be_bytes(), "pin (dora): lport big-endian");
    assert!(rbuf[52..n].ends_with(OFFER_MARKER), "pin (dora): the Offer payload verbatim");

    // Teardown: the lease's post-DORA `remove %I %I` (main.c:963) clears
    // the marker; the convs close.
    ipifc::ctl(ifc_id, b"remove", &[b"0.0.0.0", b"0.0.0.0"]).expect("pin (dora): drop the marker");
    assert!(!crate::devip::selftab_acceptall(), "pin (dora): accept-all clears post-lease");
    ipifc::ctl(ifc_id, b"unbind", &[]).expect("pin (dora): unbind");
    conv::conv_close(conv::Proto::Ipifc, ifc_id);
}

// ---------------------------------------------------------------------------
// 08-07 Task 2 — the NETIP-06 IPv6 pins: the icmpv6 full-packet bridge
// (P-E), the 40-byte ICMP_USEAD6 headers prefix (write-side), and the
// ipifc v6 verbs (add6/ra6/try + the tentative DAD lifecycle).
// ---------------------------------------------------------------------------

/// The scripted v6 guest/gateway addresses (the Medium::Ip scripted core).
const GUEST6: smoltcp::wire::Ipv6Address =
    smoltcp::wire::Ipv6Address::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x15);
const GW6: smoltcp::wire::Ipv6Address =
    smoltcp::wire::Ipv6Address::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x2);

/// Craft a bare-IPv6 ICMPv6 echo frame (Medium::Ip — no ethernet header)
/// with smoltcp's wire codecs (the checksum computed for real).
fn icmpv6_echo_frame(
    src: smoltcp::wire::Ipv6Address,
    dst: smoltcp::wire::Ipv6Address,
    request: bool,
    ident: u16,
    seq: u16,
    payload: &[u8],
) -> Vec<u8> {
    use smoltcp::wire::{Icmpv6Packet, Icmpv6Repr, Ipv6Packet, Ipv6Repr};
    let repr = if request {
        Icmpv6Repr::EchoRequest { ident, seq_no: seq, data: payload }
    } else {
        Icmpv6Repr::EchoReply { ident, seq_no: seq, data: payload }
    };
    let mut bytes = alloc_vec(40 + repr.buffer_len());
    {
        let mut ip = Ipv6Packet::new_unchecked(&mut bytes);
        Ipv6Repr {
            src_addr: src,
            dst_addr: dst,
            next_header: IpProtocol::Icmpv6,
            payload_len: repr.buffer_len(),
            hop_limit: 255,
        }
        .emit(&mut ip);
    }
    {
        let mut pkt = Icmpv6Packet::new_unchecked(&mut bytes[40..]);
        repr.emit(&src, &dst, &mut pkt, &ChecksumCapabilities::default());
    }
    bytes
}

/// Parse a captured bare-IPv6 ICMPv6 frame: (src, dst, type, ident, seq,
/// the message payload after the 8-byte echo header).  Panics loudly on
/// a non-icmpv6 frame (a pin's parse failure IS the failure).
fn parse_icmpv6(frame: &[u8]) -> (smoltcp::wire::Ipv6Address, smoltcp::wire::Ipv6Address, u8, u16, u16, Vec<u8>) {
    let ip = smoltcp::wire::Ipv6Packet::new_checked(frame).expect("pin: ip6 parse");
    let icmp = smoltcp::wire::Icmpv6Packet::new_checked(ip.payload()).expect("pin: icmpv6 parse");
    let msg = ip.payload();
    (
        ip.src_addr(),
        ip.dst_addr(),
        icmp.msg_type().into(),
        u16::from_be_bytes([msg[4], msg[5]]),
        u16::from_be_bytes([msg[6], msg[7]]),
        msg[8..].to_vec(),
    )
}

/// Read one icmpv6 conv's (state, headers, laddr, lport, raddr) — the pin
/// inspection surface (udp_conv_snapshot's sibling).
fn icmp6_conv_snapshot(id: usize) -> (crate::devip::conv::ConvState, u8, [u8; 16], u16, [u8; 16]) {
    let node = crate::mcslock::LockNode::new();
    let t = crate::devip::conv::TABLES.lock(&node);
    let c = &t.table(crate::devip::conv::Proto::Icmp6)[id];
    (c.state, c.headers, c.laddr, c.lport, c.raddr)
}

/// The full-packet round-trip pin (P-E): a user-built IPv6 packet through
/// the bridge — the scripted iface sees the PARSED payload (the bare
/// ICMPv6 message to smoltcp's socket, the ident overwritten with the
/// conv's lport per icmpkick6); a scripted inbound echo reply surfaces to
/// the data read WITH the synthesized IP6 header (the version/ploadlen/
/// proto/src/dst fields) and the correct ident dispatch.  The conv rides
/// the loopback core over the scripted device (Medium::Ip — bare packets;
/// the ipifc model carries the guest's v6 address so find_local_ip
/// resolves).
#[test]
fn icmpv6_full_packet_round_trip_through_the_bridge() {
    let _serial = SEAM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_hooks();
    set_ticks(0);
    take_wake_log();
    {
        let mut g = stack::enter_stack();
        g.core = None;
        g.ether = None;
    }
    crate::devip::arp::test_flush();
    use crate::devip::{conv, ipifc, proto};

    // The ifc model + the scripted device: bind loopback, swap the core
    // to the scripted capture device, add the guest's v6 address (the
    // v6 update_ip_addrs arm lands it on the scripted iface).
    let ifc_id = conv::conv_alloc(conv::Proto::Ipifc).expect("pin (v6rt): ipifc clone");
    ipifc::ctl(ifc_id, b"bind", &[b"loopback"]).expect("pin (v6rt): bind");
    {
        let mut g = stack::enter_stack();
        g.core = Some(StackCore::new_scripted(ScriptedDev::new()));
    }
    ipifc::ctl(ifc_id, b"add", &[b"fec0::15", b"/64"]).expect("pin (v6rt): the v6 add");
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (v6rt): the core");
        assert_eq!(
            core.iface.ip_addrs().len(),
            1,
            "pin (v6rt): the v6 address reached the smoltcp iface"
        );
    }

    // The conv: connect to the gateway (ping -6's dial shape) — laddr
    // resolves via the v6 arm of find_local_ip.
    let id = conv::conv_alloc(conv::Proto::Icmp6).expect("pin (v6rt): icmpv6 clone");
    let _guard = ConvGuard(conv::Proto::Icmp6, id);
    proto::ctl_write(conv::Proto::Icmp6, id, b"connect fec0::2!1").expect("pin (v6rt): connect");
    let (state, _h, laddr, lport, raddr) = icmp6_conv_snapshot(id);
    assert_eq!(state, conv::ConvState::Connected, "pin (v6rt): Connected");
    assert_eq!(raddr, GW6.octets(), "pin (v6rt): the recorded raddr");
    assert_eq!(laddr, GUEST6.octets(), "pin (v6rt): laddr resolved via the v6 ifc");
    assert!(lport != 0, "pin (v6rt): the ident port assigned");

    // The write: the user's FULL IPv6 packet (a zeroed 40-byte header
    // area — ping.c's memset shape — + the EchoRequestV6 with a cosmetic
    // user ident the kernel OVERWRITES).
    let mut wbuf = alloc_vec(40);
    wbuf.extend_from_slice(&[128, 0, 0xde, 0xad]); // type 128, code 0, junk cksum
    wbuf.extend_from_slice(&0x1234u16.to_be_bytes()); // user ident (overwritten)
    wbuf.extend_from_slice(&0x0007u16.to_be_bytes()); // seq 7
    wbuf.extend_from_slice(b"pin6-payload");
    let n = proto::icmp6_write_data(id, &wbuf).expect("pin (v6rt): the write");
    assert_eq!(n as usize, wbuf.len(), "pin (v6rt): the write count");
    // The scripted capture: one bare-IPv6 frame, guest → gateway, proto
    // 58, the message's ident = the conv's lport (the kernel overwrite),
    // the payload verbatim, and a VALID checksum (smoltcp recomputed).
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (v6rt): the core");
        assert_eq!(core.dev_scripted().tx_log.len(), 1, "pin (v6rt): one frame out");
        let (src, dst, ty, ident, seq, data) = parse_icmpv6(&core.dev_scripted().tx_log[0]);
        assert_eq!(src, GUEST6, "pin (v6rt): src = the iface's v6 (smoltcp's selection)");
        assert_eq!(dst, GW6, "pin (v6rt): dst = the conv's raddr");
        assert_eq!(ty, 128, "pin (v6rt): EchoRequestV6");
        assert_eq!(ident, lport, "pin (v6rt): the ident overwrite with the conv's lport");
        assert_eq!(seq, 7, "pin (v6rt): the seq verbatim");
        assert_eq!(data, b"pin6-payload", "pin (v6rt): the payload verbatim");
    }

    // The read: the scripted echo REPLY (gateway → guest, the request's
    // ident) surfaces with the synthesized 40-byte IP6 header.
    push_rx(icmpv6_echo_frame(GW6, GUEST6, false, lport, 7, b"pin6-payload"));
    let mut rbuf = alloc_vec(4096);
    let n = proto::icmp6_read_data(id, &mut rbuf).expect("pin (v6rt): the read") as usize;
    assert_eq!(n, 40 + 8 + 12, "pin (v6rt): the full-packet read (header + message)");
    assert_eq!(rbuf[0] >> 4, 6, "pin (v6rt): the synthesized header's version 6");
    let plen = u16::from_be_bytes([rbuf[4], rbuf[5]]);
    assert_eq!(plen as usize, 8 + 12, "pin (v6rt): ploadlen = the message length");
    assert_eq!(rbuf[6], 58, "pin (v6rt): next header ICMPv6");
    assert_eq!(&rbuf[8..24], &GW6.octets(), "pin (v6rt): src = the sender");
    assert_eq!(&rbuf[24..40], &GUEST6.octets(), "pin (v6rt): dst = the conv's local");
    assert_eq!(rbuf[40], 129, "pin (v6rt): EchoReplyV6");
    assert_eq!(&rbuf[44..46], &lport.to_be_bytes(), "pin (v6rt): the ident dispatch");
    assert_eq!(&rbuf[48..n], b"pin6-payload", "pin (v6rt): the payload verbatim");

    // Teardown.
    ipifc::ctl(ifc_id, b"remove", &[b"fec0::15", b"/64"]).expect("pin (v6rt): the v6 remove");
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (v6rt): the core");
        assert!(core.iface.ip_addrs().is_empty(), "pin (v6rt): the remove dropped the smoltcp address");
    }
    ipifc::ctl(ifc_id, b"unbind", &[]).expect("pin (v6rt): unbind");
    conv::conv_close(conv::Proto::Ipifc, ifc_id);
}

/// The headers-mode prefix pin (ICMP_USEAD6, the WRITE-side 40-byte
/// {unused[8], laddr[16], raddr[16]} — the donor's icmpkick6 consumes it;
/// the read side never carries it): a headers conv's write routes by the
/// PREFIX's raddr (not the conv's — dialicmp's RA-listener shape), the
/// message passes UNTOUCHED (no ident overwrite — the headers arm), and
/// the ctl's exact error strings hold.
#[test]
fn icmpv6_headers_mode_prefix_shape() {
    let _serial = SEAM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_hooks();
    set_ticks(0);
    take_wake_log();
    {
        let mut g = stack::enter_stack();
        g.core = None;
        g.ether = None;
    }
    crate::devip::arp::test_flush();
    use crate::devip::{conv, ipifc, proto};

    let ifc_id = conv::conv_alloc(conv::Proto::Ipifc).expect("pin (v6hdr): ipifc clone");
    ipifc::ctl(ifc_id, b"bind", &[b"loopback"]).expect("pin (v6hdr): bind");
    {
        let mut g = stack::enter_stack();
        g.core = Some(StackCore::new_scripted(ScriptedDev::new()));
    }
    ipifc::ctl(ifc_id, b"add", &[b"fec0::15", b"/64"]).expect("pin (v6hdr): the v6 add");

    let id = conv::conv_alloc(conv::Proto::Icmp6).expect("pin (v6hdr): icmpv6 clone");
    let _guard = ConvGuard(conv::Proto::Icmp6, id);
    // The ctl surface: an unknown verb errors EXACTLY "unknown control
    // request"; `headers` sets 6.
    let e = proto::ctl_write(conv::Proto::Icmp6, id, b"bogus").expect_err("pin (v6hdr): the unknown verb errors");
    assert_eq!(e, "unknown control request", "pin (v6hdr): the exact string");
    proto::ctl_write(conv::Proto::Icmp6, id, b"headers").expect("pin (v6hdr): headers");
    assert_eq!(icmp6_conv_snapshot(id).1, 6, "pin (v6hdr): headers=6 recorded");

    // dialicmp's shape: connect <mcast>!<type>!r <type> — the conv's
    // raddr is the multicast; a headers-mode write's PREFIX raddr steers
    // the message (the donor's headers arm).  Here: connect to the
    // all-nodes address, then write an echo request whose PREFIX names
    // the gateway.
    let allnodes: [u8; 16] = {
        let mut a = [0u8; 16];
        a[0] = 0xff;
        a[1] = 0x02;
        a[15] = 1;
        a
    };
    let _ = allnodes; // (the connect target is incidental to the prefix steering)
    proto::ctl_write(conv::Proto::Icmp6, id, b"connect fec0::99!1").expect("pin (v6hdr): connect");
    // The write: the 40-byte prefix (unused zeros, laddr = the guest,
    // raddr = the GATEWAY — steering away from the conv's fec0::99) +
    // the bare EchoRequestV6 whose ident must pass UNTOUCHED (headers
    // mode skips the donor's overwrite).
    let mut wbuf = alloc_vec(40);
    wbuf[8..24].copy_from_slice(&GUEST6.octets()); // laddr
    wbuf[24..40].copy_from_slice(&GW6.octets()); // raddr — the prefix steers
    wbuf.extend_from_slice(&[128, 0, 0, 0]); // type 128, code 0, zero cksum
    wbuf.extend_from_slice(&0x55aau16.to_be_bytes()); // the user's ident — UNTOUCHED
    wbuf.extend_from_slice(&0x0003u16.to_be_bytes()); // seq 3
    wbuf.extend_from_slice(b"hdr6");
    let n = proto::icmp6_write_data(id, &wbuf).expect("pin (v6hdr): the write");
    assert_eq!(n as usize, wbuf.len(), "pin (v6hdr): the write count");
    {
        let g = stack::enter_stack();
        let core = g.core.as_ref().expect("pin (v6hdr): the core");
        assert_eq!(core.dev_scripted().tx_log.len(), 1, "pin (v6hdr): one frame out");
        let (src, dst, ty, ident, seq, data) = parse_icmpv6(&core.dev_scripted().tx_log[0]);
        assert_eq!(dst, GW6, "pin (v6hdr): the PREFIX's raddr steered the message");
        assert_eq!(src, GUEST6, "pin (v6hdr): src = smoltcp's RFC 6724 selection (the recorded corner)");
        assert_eq!(ty, 128, "pin (v6hdr): EchoRequestV6");
        assert_eq!(ident, 0x55aa, "pin (v6hdr): the headers-mode ident is UNTOUCHED");
        assert_eq!(seq, 3, "pin (v6hdr): the seq verbatim");
        assert_eq!(data, b"hdr6", "pin (v6hdr): the payload verbatim");
    }

    ipifc::ctl(ifc_id, b"remove", &[b"fec0::15", b"/64"]).expect("pin (v6hdr): remove");
    ipifc::ctl(ifc_id, b"unbind", &[]).expect("pin (v6hdr): unbind");
    conv::conv_close(conv::Proto::Ipifc, ifc_id);
}

/// The ipifc v6 verbs pin: `add6`'s EUI-64 derivation on an ether ifc
/// (ethermedium's pref2addr), the exact donor error strings, `ra6`'s
/// keyword recording (the status line prints it), and `try`'s tentative
/// lifecycle (recorded, NEVER pushed to smoltcp until the confirming
/// add — the DAD window's shape).
#[test]
fn ipifc_v6_verbs_add6_ra6_try() {
    let _serial = SEAM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    ensure_hooks();
    set_ticks(0);
    take_wake_log();
    {
        let mut g = stack::enter_stack();
        g.core = None;
        g.ether = None;
    }
    crate::devip::arp::test_flush();
    use crate::devip::{conv, ipifc};

    // add6 on a MAC-less medium errors "no pref2addr on interface"
    // (the loopback Medium has no pref2addr — the donor's arm).
    let lo_id = conv::conv_alloc(conv::Proto::Ipifc).expect("pin (verbs): ipifc clone");
    ipifc::ctl(lo_id, b"bind", &[b"loopback"]).expect("pin (verbs): bind loopback");
    let e = ipifc::ctl(lo_id, b"add6", &[b"fec0::", b"64"]).expect_err("pin (verbs): add6 on loopback errors");
    assert_eq!(e, "no pref2addr on interface", "pin (verbs): the pref2addr arm");
    // The arg-count + parse errors fire before the MAC lookup.
    let e = ipifc::ctl(lo_id, b"add6", &[]).expect_err("pin (verbs): add6 with no args errors");
    assert_eq!(e, "bad arg in system call", "pin (verbs): Ebadarg");
    let e = ipifc::ctl(lo_id, b"add6", &[b"10.0.0.1"]).expect_err("pin (verbs): a v4 prefix errors");
    assert_eq!(e, "bad ipv6 address", "pin (verbs): the not-v6 arm");
    let e = ipifc::ctl(lo_id, b"add6", &[b"fe80::1", b"64"]).expect_err("pin (verbs): link-local errors");
    assert_eq!(e, "ipv6 prefix is link-local", "pin (verbs): the link-local arm");
    let e = ipifc::ctl(lo_id, b"add6", &[b"fec0::", b"64", b"1", b"1", b"5", b"9"]).expect_err("pin (verbs): validlt<preflt errors");
    assert_eq!(e, "valid ipv6 lifetime less than preferred lifetime", "pin (verbs): the lifetime arm");
    ipifc::ctl(lo_id, b"unbind", &[]).expect("pin (verbs): unbind loopback");
    conv::conv_close(conv::Proto::Ipifc, lo_id);

    // The ether leg: the scripted ether core + the test ifc record (the
    // live netif machinery is absent on the host — the pin's seam).
    crate::devip::stack::install_test_ether(StackCore::new_scripted_ether(
        crate::devip::phyadapt::ScriptedEtherDev::new(GUEST_MAC),
    ));
    let et_id = conv::conv_alloc(conv::Proto::Ipifc).expect("pin (verbs): ether ipifc clone");
    crate::devip::ipifc::test_bind_ether(et_id);

    // add6 fec0:: 64 → the EUI-64 derivation of the scripted MAC
    // (52:54:00:12:34:56 → 5254:00ff:fe12:3456 — the U/L bit (0x02) is
    // already set in 0x52, so the |0x2 leaves the first octet unchanged).
    ipifc::ctl(et_id, b"add6", &[b"fec0::", b"64"]).expect("pin (verbs): add6");
    let eui64: [u8; 16] = [
        0xfe, 0xc0, 0, 0, 0, 0, 0, 0, 0x52, 0x54, 0x00, 0xff, 0xfe, 0x12, 0x34, 0x56,
    ];
    {
        let g = stack::enter_stack();
        let core = g.core_ref(stack::CoreSel::Ether).expect("pin (verbs): the ether core");
        assert!(
            core.iface.ip_addrs().iter().any(|a| matches!(a, IpCidr::Ipv6(c) if c.address() == smoltcp::wire::Ipv6Address::from(eui64))),
            "pin (verbs): the EUI-64 address reached the ether iface — got {:?}",
            core.iface.ip_addrs()
        );
    }

    // ra6: record + the status line prints it; the error arms.
    let e = ipifc::ctl(et_id, b"ra6", &[b"recvra"]).expect_err("pin (verbs): an odd field count errors");
    assert_eq!(e, "bad arg in system call", "pin (verbs): the odd-count arm");
    let e = ipifc::ctl(et_id, b"ra6", &[b"bogus", b"1"]).expect_err("pin (verbs): an unknown keyword errors");
    assert_eq!(e, "bad arg in system call", "pin (verbs): the unknown-keyword arm");
    ipifc::ctl(et_id, b"ra6", &[b"maxraint", b"100", b"minraint", b"200"]).expect_err("pin (verbs): maxraint<minraint errors");
    ipifc::ctl(et_id, b"ra6", &[b"recvra", b"1", b"mflag", b"1", b"maxraint", b"700000", b"ttl", b"64"])
        .expect("pin (verbs): the ra6 record");
    let status = ipifc::status(et_id);
    let text = std::string::String::from_utf8_lossy(&status).into_owned();
    assert!(text.contains("sendra 0 recvra 1"), "pin (verbs): the status line carries recvra: {text:?}");
    assert!(text.contains("mflag 1"), "pin (verbs): mflag recorded");
    assert!(text.contains("maxraint 700000"), "pin (verbs): maxraint recorded");
    assert!(text.contains("ttl 64"), "pin (verbs): ttl recorded");
    // The maxraint<minraint restore held: the earlier rejected write
    // restored the defaults (minraint 200000 < maxraint 700000 now).
    assert!(text.contains("minraint 200000"), "pin (verbs): the rejected pair was restored: {text:?}");

    // try: the tentative lifc is recorded (the DAD window's model
    // visibility) but NEVER pushed to smoltcp; the confirming add pushes
    // it (the DAD tail).
    let try6 = "fe80::9"; // link-local is FINE for try (the link-local
                          // bar is add6's; ipifcadd has no such check)
    let try6_addr: [u8; 16] = [0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9];
    ipifc::ctl(et_id, b"try", &[try6.as_bytes(), b"/64"]).expect("pin (verbs): try");
    {
        let g = stack::enter_stack();
        let core = g.core_ref(stack::CoreSel::Ether).expect("pin (verbs): the ether core");
        assert!(
            !core.iface.ip_addrs().iter().any(|a| matches!(a, IpCidr::Ipv6(c) if c.address() == smoltcp::wire::Ipv6Address::from(try6_addr))),
            "pin (verbs): a tentative address is NEVER pushed to smoltcp (the DAD window)"
        );
    }
    // The tentative is visible in the model (the status line's per-address
    // list carries it).
    let status = ipifc::status(et_id);
    let text = std::string::String::from_utf8_lossy(&status).into_owned();
    assert!(text.contains("fe80::9"), "pin (verbs): the tentative lifc is model-visible: {text:?}");
    // The confirming add (the quiet DAD window's tail): pushed now.
    ipifc::ctl(et_id, b"add", &[try6.as_bytes(), b"/64"]).expect("pin (verbs): the confirming add");
    {
        let g = stack::enter_stack();
        let core = g.core_ref(stack::CoreSel::Ether).expect("pin (verbs): the ether core");
        assert!(
            core.iface.ip_addrs().iter().any(|a| matches!(a, IpCidr::Ipv6(c) if c.address() == smoltcp::wire::Ipv6Address::from(try6_addr))),
            "pin (verbs): the confirming add pushed the address"
        );
    }

    // Teardown: drop the ifc record + close (the scripted ether core is
    // the pin's own; the next pin's fresh_scripted_stack clears it).
    crate::devip::ipifc::test_unbind_ether();
    conv::conv_close(conv::Proto::Ipifc, et_id);
}

