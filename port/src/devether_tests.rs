//! The devether/netif pins: homed at the CRATE root beside
//! allocator_tests.rs/virtio_tests.rs (the crate-root placement
//! precedent — lib.rs's registration block).
//!
//! readnum_content's three byte-exact pins record the donor's 32-bit
//! truncation — devcons.c:712's `ulong` value parameter against
//! netif.h:40's `int type` field (the G-07-2 oracle): `connect -1`
//! reads back as ` 4294967295 ` (12 bytes), never a usize underflow,
//! never a sign-extended 64-bit value, never a clamp.  Plan 9
//! long/ulong is 32-bit even on amd64 (amd64/include/u.h); vlong/uvlong
//! are the 64-bit types, hence VLNUMSIZE=22 where NUMSIZE=12 cannot
//! hold a 64-bit value.
//!
//! The conv_close pin records the donor's last-close reset (9k
//! netif.c:446 — typ=0, owner cleared, queue drain, EOF wake) as
//! INTENTIONAL donor behavior (G-07-1), so a future reader cannot
//! 'repair' it back into the misdiagnosed kernel defect: the UAT
//! test-1 failure was the PROCEDURE (connect before holding the
//! conversation open), never the kernel.

use std::sync::Mutex;

use crate::devether::readnum_content;
use crate::mcslock::LockNode;
use crate::netif::{self, Pop};

#[test]
fn readnum_content_neg1_matches_donor_truncation() {
    // The donor's conversion: Netfile.type (a 32-bit int, netif.h:40)
    // through readnum's 32-bit ulong value parameter (devcons.c:712) —
    // truncating, never sign-extending.  -1 → 4294967295 (10 digits) →
    // one pad space → the trailing space: exactly 12 bytes.
    assert_eq!(readnum_content(-1), b" 4294967295 ".to_vec());
}

#[test]
fn readnum_content_zero_pads_numsize() {
    // 0 → one digit → ten pad spaces → trailing space (NUMSIZE=12).
    assert_eq!(readnum_content(0), b"          0 ".to_vec());
}

#[test]
fn readnum_content_max_ethertype_65535() {
    // The max admitted ether-type neighbor (write_ctl's 0xffff
    // ceiling): five digits → six pad spaces → trailing space.
    assert_eq!(readnum_content(65535), b"      65535 ".to_vec());
}

/// The stub EtherHooks for the conv_close pin: plain fn pointers over
/// test-module statics (host tests run under std — port is
/// cfg_attr(not(test), no_std)).  current() returns a fixed NONZERO pid
/// (WaitSet.register silently drops pid==0, netif.rs:328); block() is a
/// no-op (the pin drives the close directly — nothing actually parks);
/// wake() records the pid slice into the log.
const STUB_PID: u64 = 42;
static WAKE_LOG: Mutex<Vec<u64>> = Mutex::new(Vec::new());

fn stub_current() -> u64 {
    STUB_PID
}

fn stub_block() {}

fn stub_wake(pids: &[u64]) {
    WAKE_LOG.lock().unwrap().extend_from_slice(pids);
}

#[test]
fn conv_close_last_close_reset_is_donor_intentional() {
    // G-07-1's pin: the donor's netifclose resets the conversation at
    // the last close (f->type = 0, qclose(f->iq) — 9k netif.c:446/449)
    // and wakes any blocked reader to EOF.  This test passes TODAY — it
    // pins existing donor-faithful behavior as intentional.
    //
    // publish asserts the interface is published at most once per
    // process — this test is the ONLY publish caller in the test binary
    // (the readnum pins never publish).  The MAC is the guest's
    // negotiated address from the virtnet gates — a UNICAST address:
    // rx_ingress's multicast gate (frame[0] & 1) drops before demux
    // when prom == 0 (netif.rs:593).
    netif::publish([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);

    // conv_alloc FIRST and conv_open deliberately ABSENT: conv_alloc is
    // the ONLY store allocator (netif.rs:417-419) — conv_open errors
    // "devether: no such conversation" on a store-less slot
    // (netif.rs:444-446), and its inuse bump would keep the close above
    // the reset arm.  inuse arithmetic: 1 after alloc (netif.rs:420),
    // untouched by conv_connect, decremented 1→0 by conv_close
    // (netif.rs:487-488) — so the close reaches the inuse==0 reset arm.
    let id = netif::conv_alloc().expect("conv_alloc");
    assert_eq!(id, 0);
    netif::conv_connect(id, 2054).expect("conv_connect");

    // One ethertype-2054 frame: dest = the published MAC, bytes 12-13 =
    // 0x08,0x06 (2054 big-endian) — delivered to conv 0 because
    // slot.open && slot.typ == 2054 && tome (netif.rs:598-613).
    let mut frame = [0u8; 14];
    frame[..6].copy_from_slice(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
    frame[6..12].copy_from_slice(&[0x52, 0x54, 0x00, 0x00, 0x00, 0x01]);
    frame[12] = 0x08;
    frame[13] = 0x06;

    let mut buf = [0u8; 1514];
    netif::rx_ingress(&frame);
    // Delivery PROVEN — without this probe the post-close drain
    // assertion is vacuous: a frame that never landed drains trivially.
    assert!(matches!(netif::pop_frame(id, &mut buf), Pop::Frame(_)));

    netif::rx_ingress(&frame); // the queue now holds one frame at close time

    // Stub hooks BEFORE conv_close (the close wakes through them); the
    // waiter registers AFTER the pushes so rx_ingress's waiter-take
    // (netif.rs:624-625) cannot consume the registration the EOF-wake
    // assertion depends on.
    netif::set_hooks(netif::EtherHooks {
        current: stub_current,
        block: stub_block,
        wake: stub_wake,
    });
    netif::register_rx_waiter(stub_current());

    // The last close: inuse 1→0 — the donor's reset arm fires
    // (netif.rs:490-508).
    netif::conv_close(id);

    // (i) The canonical record reset, read through the NETIF pub(crate)
    // chain (NETIF → convs → fields) under the LockNode idiom
    // conv_close itself uses.
    {
        let node = LockNode::new();
        let ng = netif::NETIF.lock(&node);
        let nif = ng.as_ref().expect("published");
        assert_eq!(nif.convs[id].typ, 0);
        assert!(!nif.convs[id].owner_set);
        assert_eq!(nif.convs[id].inuse, 0);
    }

    // (ii) The EOF wake: the close drained the rx waiter set and woke
    // the registered reader through the stub hook.
    assert_eq!(WAKE_LOG.lock().unwrap().as_slice(), &[STUB_PID]);

    // (iii) Queue-drain AND open==false observed BEHAVIORALLY, never by
    // field reads — the INGRESS-side ConvSlot/FrameStore/IngressState
    // fields are module-private (netif.rs:221/268/285-286 carry no
    // pub), so pop_frame is the surface: it pops the store BEFORE the
    // open check (netif.rs:685-690), so an undrained queue returns
    // Pop::Frame, a still-open slot returns Pop::Empty — Closed ==
    // drained + closed.
    assert!(matches!(netif::pop_frame(id, &mut buf), Pop::Closed));
}

// ---------------------------------------------------------------------------
// WR-05 capacity pins (17-03): the WaitSet must hold EVERY possible
// waiter — one per proc, and the live proc table is 512 (MAX_PROCS at
// x86_64/src/proc.rs:112 and aarch64/src/proc.rs:73) — and the full arm
// must be LOUD (counted + reported, never a silent drop, never a
// panic).  Pure-struct pins: each constructs the set directly and never
// touches the published device (Pitfall 8 — the publish-once discipline
// above stays the single publish's only consumer).
// ---------------------------------------------------------------------------

/// The table-sized capacity: 512 distinct nonzero pids all register and
/// are ALL returned by take — none silently dropped (the 16-slot arrays
/// fail this today; that is the RED).
#[test]
fn netif_waitset_retains_the_whole_proc_table_and_take_returns_all() {
    let mut ws = crate::netif::WaitSet::new();
    for pid in 1..=512u64 {
        ws.register(pid);
    }
    let mut out = [0u64; crate::MAX_WAITERS];
    let n = ws.take(&mut out);
    assert_eq!(n, 512, "512 distinct live pids all register (the set is sized to the table)");
    let mut seen: Vec<u64> = out[..n].to_vec();
    seen.sort_unstable();
    assert_eq!(seen, (1..=512).collect::<Vec<u64>>(), "no pid silently dropped");
}

/// The 513th distinct registration hits the full arm: the table-sized
/// prefix survives, the beyond-capacity registration is dropped —
/// LOUDLY (counted + reported), never a panic, never silence.
#[test]
fn netif_waitset_513th_registration_is_loudly_dropped() {
    let mut ws = crate::netif::WaitSet::new();
    for pid in 1..=513u64 {
        ws.register(pid);
    }
    let mut out = [0u64; crate::MAX_WAITERS];
    let n = ws.take(&mut out);
    assert_eq!(n, 512, "the table-sized prefix survives; only the beyond-capacity 513th drops");
    // The loud arm's observable: the beyond-capacity drop is counted
    // (and reported on the console in-kernel) — never silent.
    assert_eq!(
        ws.dropped(),
        1,
        "the 513th registration's drop is counted and reported, not silent"
    );
}

/// The register contract below capacity is unchanged: duplicates stay
/// idempotent (one slot, never two), pid 0 stays ignored (the empty-slot
/// marker), and take drains.
#[test]
fn netif_waitset_duplicate_and_zero_registrations_stay_idempotent() {
    let mut ws = crate::netif::WaitSet::new();
    ws.register(7);
    ws.register(7); // duplicate — one slot, never two
    ws.register(0); // pid 0 marks an empty slot: ignored by contract
    let mut out = [0u64; crate::MAX_WAITERS];
    let n = ws.take(&mut out);
    assert_eq!(n, 1, "duplicate and zero registrations add nothing");
    assert_eq!(out[0], 7);
    let n2 = ws.take(&mut out);
    assert_eq!(n2, 0, "take drains the set");
    assert_eq!(ws.dropped(), 0, "below capacity nothing is ever dropped");
}
