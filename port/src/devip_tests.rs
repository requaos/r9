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
    let (.., lport) = {
        let s = udp_conv_snapshot(id);
        (s.0, s.1, s.2, s.3, s.4, s.5, s.6, s.7, s.8)
    };
    assert_eq!(lport, 7, "pin (e): announce bound port 7");
    assert_eq!(udp_conv_snapshot(id).1, 7, "pin (e): headers=7 recorded");

    push_rx(udp4_frame(PEER, GUEST, 4321, 7, b"pin payload"));
    stack::drive_stack();

    let mut buf = [0u8; 128];
    let n = crate::devip::proto::udp_read_data(id, &mut buf).expect("pin (e): read delivers");
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
    crate::devip::proto::udp_rx_drain();
    assert_eq!(udp_conv_snapshot(id).7, 1, "pin (spawn): no second spawn for a known quad");
    assert_eq!(udp_conv_snapshot(nid).8, 2, "pin (spawn): the repeat datagram queued on the same conv");

    // New quad → another spawn.
    push_rx(udp4_frame(PEER2, GUEST, 5001, 7, b"three"));
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
    let n = crate::devip::proto::udp_read_data(nid, &mut buf).expect("pin (spawn): spawned read");
    assert_eq!(&buf[..n], b"one", "pin (spawn): first datagram first");
    let n = crate::devip::proto::udp_read_data(nid, &mut buf).expect("pin (spawn): spawned read");
    assert_eq!(&buf[..n], b"two", "pin (spawn): second datagram next");

    // Close out the spawned convs (the announcer's guard covers its own
    // close; incall convs are closed by the announcer's last close).
    let _ = nc;
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
    let (.., blport) = {
        let s = udp_conv_snapshot(b);
        (s.0, s.1, s.2, s.3, s.4, s.5, s.6, s.7, s.8)
    };
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
    let n = crate::devip::proto::udp_read_data(b, &mut buf).expect("pin (wedge): B reads");
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
