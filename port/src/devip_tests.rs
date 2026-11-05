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
use std::sync::atomic::{AtomicU64, Ordering};

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

fn stub_block() {}

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
    let mut pool = stack::AcceptPool::announce(2, 7).expect("pin: announce");
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
