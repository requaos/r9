// The kernel-side ARP trigger for qemu's q35 net gate (13-05 Task 3,
// X64-04 + X64-02's load proof) — the Phase-7 payload machinery re-keyed,
// not reinvented: the committed 60-byte ARP request fixture
// (build/aarch64/root-tree/lib/test/arp.req — ETHERMINTU-padded 42+18,
// sender 10.0.2.15, target the gateway 10.0.2.2 because slirp's ARP
// responder answers only its own addresses — the 07 erratum + watch-out
// carried verbatim), posted through devether's published driver ops
// (the widened pub driver_ops() seam — the Open-Question-1 resolution:
// kernel-side ONLY this phase, the smallest seam, no exec/bootchain
// machinery until Phase 14's XEXEC).
//
// Two stages, reconciled against the driver's ifstats tuples (the
// Phase-7 D-08 shape):
//  (a) PING-PONG xN — post one frame, wait for its RX retire, repeat.
//      Exact reconcile: RX used == N and TX used == N exactly, nintr
//      within [1, N] per queue.
//  (b) TX-BULK — post in a tight loop until the ring-full backpressure
//      path (tx_post -> Ok(false)) exercises the bounded wait-for-retire
//      shape, a bulk total in the thousands.  Bounds with no wedge: TX
//      deltas exact; the gateway's replies keep riding RX under load
//      (printed, the X64-02 load observation).
// Any mismatch prints `netgate: FAIL <stage>` and the terminal marker
// never lands — the gate reds, never a silent pass (T-13-12: the
// reconciliation is honest and fail-closed).
//
// Concurrency: run() executes from boot context AFTER sti, so the
// virtio IRQ handler CAN interrupt it.  The aarch64 bind's writer
// contract (handler vs driver ops mutually exclusive in TIME) is
// preserved the same way the donor kernel does it: every tx_post call
// runs with interrupts masked — a cli..sti window around the post (the
// masked-syscall invariant, x86 edition).  The waits run enabled, so
// the retire interrupts the wait is FOR can arrive.

use crate::cpu;
use crate::platform::q35::timer;

#[cfg(not(test))]
use port::println;

/// The ping-pong count (the plan's pin: a few hundred — CI-scale N per
/// the flagged assumption; the pcap assertion derives its floor from
/// the kernel's own `ping-pong done N/N` marker, never a second
/// constant).
pub const PINGPONG_N: u32 = 256;
/// The TX-bulk total: thousands of posts through the 128-slot ring, so
/// the backpressure path (ring full -> bounded wait for retire) is
/// exercised dozens of times over.
pub const TXBULK_N: u32 = 4096;
/// Per-reply wait bound, in 100 Hz timer ticks (3 s each — generous for
/// a TCG leg; the IRQ path under load still lands well inside).
const WAIT_TICKS: u64 = 300;

/// println with interrupts masked: the netgate runs at boot context
/// AFTER sti, and the console's MCS lock is not preempt-safe against an
/// interrupted holder — the timer tick prints once a second from IRQ
/// context, and an IRQ-context locker whose predecessor is this
/// boot-context holder spins forever (the "no console on the hot path"
/// discipline's mirror side: a post-sti boot-context printer must
/// mask).  The window is microseconds (the emulated 16550 accepts
/// bytes immediately).
#[cfg(dev_ether)]
macro_rules! masked_println {
    ($($arg:tt)*) => {{
        crate::cpu::cli();
        port::println!($($arg)*);
        crate::cpu::sti();
    }};
}

/// The baked ARP request — the committed Phase-7 fixture's 60 bytes
/// verbatim (sender 10.0.2.15 -> the gateway 10.0.2.2, ETHERMINTU pad).
/// The ethernet source (bytes 6..12) is overwritten with the NEGOTIATED
/// MAC before every post (devether's tx_post contract: "source MAC
/// already overwritten" — the caller's job).
#[cfg(any(test, dev_ether))]
const ARP_REQUEST: [u8; 60] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // dst: broadcast
    0x52, 0x54, 0x00, 0x12, 0x34, 0x56, // src: overwritten with ops.mac
    0x08, 0x06, // ethertype: ARP
    0x00, 0x01, // htype: Ethernet
    0x08, 0x00, // ptype: IPv4
    6, 4, // hlen, plen
    0x00, 0x01, // oper: request
    0x52, 0x54, 0x00, 0x12, 0x34, 0x56, // sha (the fixture's)
    10, 0, 2, 15, // spa: the slirp guest address
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // tha
    10, 0, 2, 2, // tpa: the slirp GATEWAY (the watch-out)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, // the 18-byte ETHERMINTU pad
];

/// One ifstats snapshot: (avail, used, lastused, nintr, nnote) per
/// queue — vq0 RX, vq1 TX (EtherDriverOps::ifstats' shape).
type Stats = [(u16, u16, u16, u32, u32); 2];

/// Progress of a wrapping u16 ring position over a base, as u32 (the
/// reconciliation arithmetic — wrap-safe at these counts, exact under
/// 65536 cumulative frames).
#[cfg(any(test, dev_ether))]
fn progress(now: u16, base: u16) -> u32 {
    now.wrapping_sub(base) as u32
}

/// The ping-pong stage's exact reconciliation (the D-08 shape): RX used
/// == TX used == n exactly; nintr within [1, n] on both queues (an
/// interrupt observed the work at least once, never more interrupts
/// than work); both rings drained (lastused == used).  None is a FAIL
/// naming the mismatched field.
#[cfg(any(test, dev_ether))]
fn reconcile_pingpong(base: &Stats, now: &Stats, n: u32) -> Result<(), &'static str> {
    let rx = progress(now[0].1, base[0].1);
    if rx != n {
        return Err("rx used count != N");
    }
    let tx = progress(now[1].1, base[1].1);
    if tx != n {
        return Err("tx used count != N");
    }
    if now[0].2 != now[0].1 {
        return Err("rx ring undrained at reconcile");
    }
    if now[1].2 != now[1].1 {
        return Err("tx ring undrained at reconcile");
    }
    if now[0].3 < 1 || now[0].3 > n {
        return Err("rx nintr outside [1, N]");
    }
    if now[1].3 < 1 || now[1].3 > n {
        return Err("tx nintr outside [1, N]");
    }
    Ok(())
}

/// The TX-bulk stage's bounds reconciliation: TX used == n exactly (the
/// drain caught up — no wedge); RX used >= 0 is printed as-is (the
/// gateway's replies ride the load — the X64-02 observation, not an
/// exactness contract); nintr within [1, tx_total] on both queues.
#[cfg(any(test, dev_ether))]
fn reconcile_txbulk(base: &Stats, now: &Stats, n: u32) -> Result<u32, &'static str> {
    let tx = progress(now[1].1, base[1].1);
    if tx != n {
        return Err("tx used count != bulk total");
    }
    if now[1].2 != now[1].1 {
        return Err("tx ring undrained at reconcile (wedge)");
    }
    if now[0].3 < 1 {
        return Err("rx nintr zero under load");
    }
    if now[1].3 < 1 {
        return Err("tx nintr zero under load");
    }
    Ok(progress(now[0].1, base[0].1))
}

/// The trigger entry: run both stages from boot context after sti.  A
/// driver that never published (or any failure) prints loudly and
/// returns — boot continues to the idle loop; the gate reds on the
/// missing markers (fail-closed, T-13-12).
#[cfg(dev_ether)]
pub fn run() {
    let ops = match port::devether::driver_ops() {
        Ok(ops) => ops,
        Err(msg) => {
            masked_println!("netgate: {msg} — trigger skipped (no published driver)");
            return;
        }
    };
    let mut frame = ARP_REQUEST;
    frame[6..12].copy_from_slice(&ops.mac);

    // ---- stage (a): ping-pong xN --------------------------------------
    let base = (ops.ifstats)();
    for i in 0..PINGPONG_N {
        // The masked-post window (the writer contract's TIME exclusion):
        // cli around the post so the IRQ handler can never interleave
        // with the avail-side publication; sti before the wait so the
        // reply's interrupt can arrive.
        cpu::cli();
        let posted = (ops.tx_post)(&frame);
        cpu::sti();
        match posted {
            Ok(true) => {}
            Ok(false) => {
                // One frame per iteration with a retire between: the
                // 128-slot ring cannot be full here — a FAIL, not a wait.
                masked_println!("netgate: FAIL ping-pong ring unexpectedly full at {i}");
                return;
            }
            Err(msg) => {
                masked_println!("netgate: FAIL ping-pong post at {i}: {msg}");
                return;
            }
        }
        // Wait for THIS reply's RX retire: the device's used position
        // advances to i+1, then the IRQ drain moves lastused up.
        if !wait_rx_retired(&ops, base[0].1, i + 1) {
            masked_println!("netgate: FAIL ping-pong rx retire {}/{} (no reply inside the budget)", i + 1, PINGPONG_N);
            return;
        }
    }
    // The TX retires ride the same IRQs; wait the drain out, then
    // reconcile exactly.
    if !wait_tx_retired(&ops, base[1].1, PINGPONG_N) {
        masked_println!("netgate: FAIL ping-pong tx retire drain");
        return;
    }
    let now = (ops.ifstats)();
    if let Err(field) = reconcile_pingpong(&base, &now, PINGPONG_N) {
        masked_println!("netgate: FAIL ping-pong reconcile: {field}");
        return;
    }
    masked_println!(
        "netgate: ping-pong done {PINGPONG_N}/{PINGPONG_N} intr rx {} tx {}",
        now[0].3, now[1].3
    );
    print_ifstats(&now);

    // ---- stage (b): TX-bulk -------------------------------------------
    let base2 = (ops.ifstats)();
    let mut posted = 0u32;
    while posted < TXBULK_N {
        cpu::cli();
        let r = (ops.tx_post)(&frame);
        match r {
            Ok(true) => {
                cpu::sti();
                posted += 1;
            }
            Ok(false) => {
                // The designed backpressure: the ring is full — wait
                // (enabled) for the IRQ retire to free slots, bounded,
                // never a deadlock.
                cpu::sti();
                if !wait_tx_retired(&ops, base2[1].1, posted) {
                    masked_println!("netgate: FAIL tx-bulk backpressure retire at {posted}");
                    return;
                }
            }
            Err(msg) => {
                cpu::sti();
                masked_println!("netgate: FAIL tx-bulk post at {posted}: {msg}");
                return;
            }
        }
    }
    if !wait_tx_retired(&ops, base2[1].1, TXBULK_N) {
        masked_println!("netgate: FAIL tx-bulk retire drain");
        return;
    }
    let now2 = (ops.ifstats)();
    let rx_riding = match reconcile_txbulk(&base2, &now2, TXBULK_N) {
        Ok(rx) => rx,
        Err(field) => {
            masked_println!("netgate: FAIL tx-bulk reconcile: {field}");
            return;
        }
    };
    masked_println!("netgate: tx-bulk done {TXBULK_N} rx-under-load {rx_riding} intr rx {} tx {}", now2[0].3, now2[1].3);
    print_ifstats(&now2);

    masked_println!("X64-NET-DONE");
}

/// Wait (interrupts enabled, hlt between polls) for the RX used ring to
/// show `n` entries past `base` AND the IRQ drain to have caught up.
/// Bounded by WAIT_TICKS — false means the budget expired (a FAIL at
/// the caller, never a hang).
#[cfg(dev_ether)]
fn wait_rx_retired(ops: &port::devether::EtherDriverOps, base: u16, n: u32) -> bool {
    let start = timer::ticks();
    loop {
        let s = (ops.ifstats)();
        if progress(s[0].1, base) >= n && s[0].2 == s[0].1 {
            return true;
        }
        if timer::ticks().wrapping_sub(start) > WAIT_TICKS {
            return false;
        }
        // SAFETY: hlt faults nowhere and parks until any interrupt (the
        // reply's IRQ or the 100 Hz timer); the loop re-checks on wake.
        unsafe { core::arch::asm!("hlt") };
    }
}

/// The TX drain wait: `n` used entries past `base` with the driver's
/// drain position caught up (tx_next_slot frees slots only on the
/// drain, so backpressure waits ride this too).
#[cfg(dev_ether)]
fn wait_tx_retired(ops: &port::devether::EtherDriverOps, base: u16, n: u32) -> bool {
    let start = timer::ticks();
    loop {
        let s = (ops.ifstats)();
        if progress(s[1].1, base) >= n && s[1].2 == s[1].1 {
            return true;
        }
        if timer::ticks().wrapping_sub(start) > WAIT_TICKS {
            return false;
        }
        // SAFETY: as wait_rx_retired.
        unsafe { core::arch::asm!("hlt") };
    }
}

/// The per-queue line in the driver's ifstats file shape — the audit
/// trail every gate log carries next to the stage markers.
#[cfg(dev_ether)]
fn print_ifstats(s: &Stats) {
    for (q, t) in s.iter().enumerate() {
        masked_println!(
            "netgate: vq{q}: avail={} used={} lastused={} nintr={} nnote={}",
            t.0, t.1, t.2, t.3, t.4
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The frame is exactly the committed fixture's shape: broadcast dst,
    // ARP ethertype, request oper, spa 10.0.2.15, tpa the GATEWAY
    // 10.0.2.2 (the slirp watch-out), 60 bytes total (ETHERMINTU).
    #[test]
    fn frame_is_the_committed_fixture_shape() {
        assert_eq!(ARP_REQUEST.len(), 60);
        assert_eq!(&ARP_REQUEST[0..6], &[0xff; 6], "broadcast dst");
        assert_eq!(&ARP_REQUEST[12..14], &[0x08, 0x06], "ARP ethertype");
        assert_eq!(&ARP_REQUEST[20..22], &[0x00, 0x01], "oper = request");
        assert_eq!(&ARP_REQUEST[28..32], &[10, 0, 2, 15], "spa = the slirp guest");
        assert_eq!(&ARP_REQUEST[38..42], &[10, 0, 2, 2], "tpa = the slirp GATEWAY");
        assert_eq!(&ARP_REQUEST[42..60], &[0u8; 18], "the ETHERMINTU pad");
    }

    fn stats(rx_used: u16, rx_last: u16, rx_nintr: u32, tx_used: u16, tx_last: u16, tx_nintr: u32) -> Stats {
        [
            (rx_used, rx_used, rx_last, rx_nintr, 0),
            (tx_used, tx_used, tx_last, tx_nintr, 0),
        ]
    }

    // The exact ping-pong reconcile, both ways: the golden N==N pass,
    // and every mismatch class (lost reply, undrained ring, nintr
    // outside [1, N] — the storm signature nintr > N included) fails
    // naming its field.
    #[test]
    fn pingpong_reconcile_is_exact_and_fails_closed() {
        let base = stats(0, 0, 0, 0, 0, 0);
        // golden: N replies retired, drained, one interrupt per
        // observation batch within bounds
        let ok = stats(256, 256, 256, 256, 256, 200);
        assert_eq!(reconcile_pingpong(&base, &ok, 256), Ok(()));
        // batching is fine: fewer interrupts than frames
        let ok2 = stats(256, 256, 1, 256, 256, 37);
        assert_eq!(reconcile_pingpong(&base, &ok2, 256), Ok(()));
        // a lost reply: rx 255 != 256
        assert_eq!(reconcile_pingpong(&base, &stats(255, 255, 255, 256, 256, 10), 256), Err("rx used count != N"));
        // tx short
        assert_eq!(reconcile_pingpong(&base, &stats(256, 256, 255, 255, 255, 10), 256), Err("tx used count != N"));
        // undrained rx (the IRQ drain lagging forever = a wedge)
        let mut lag = stats(256, 256, 255, 256, 256, 10);
        lag[0].1 = 256;
        lag[0].2 = 255;
        assert_eq!(reconcile_pingpong(&base, &lag, 256), Err("rx ring undrained at reconcile"));
        // nintr == 0: work happened but no interrupt observed it
        assert_eq!(
            reconcile_pingpong(&base, &stats(256, 256, 0, 256, 256, 10), 256),
            Err("rx nintr outside [1, N]")
        );
        // nintr > N: the interrupt-storm signature (Pitfall 5's class)
        assert_eq!(
            reconcile_pingpong(&base, &stats(256, 256, 300, 256, 256, 10), 256),
            Err("rx nintr outside [1, N]")
        );
    }

    // The bulk reconcile: TX exact + drained (the no-wedge proof), the
    // rx-under-load count returned as the printed observation.
    #[test]
    fn txbulk_reconcile_bounds_and_reports_rx() {
        let base = stats(256, 256, 256, 256, 256, 200);
        let now = stats(256 + 300, 256 + 300, 500, 256 + 4096, 256 + 4096, 900);
        assert_eq!(reconcile_txbulk(&base, &now, 4096), Ok(300));
        // tx short of the bulk total
        assert_eq!(
            reconcile_txbulk(&base, &stats(600, 600, 500, 256 + 4000, 256 + 4000, 900), 4096),
            Err("tx used count != bulk total")
        );
        // undrained = the wedge the stage exists to catch
        let mut wedge = stats(600, 600, 500, 256 + 4096, 256 + 4096, 900);
        wedge[1].2 = 256 + 4090;
        assert_eq!(reconcile_txbulk(&base, &wedge, 4096), Err("tx ring undrained at reconcile (wedge)"));
    }

    // u16 wrap-safety of the progress arithmetic at the gate's scale
    // (cumulative counts stay far under 65536; the helper is exact).
    #[test]
    fn progress_is_wrap_safe() {
        assert_eq!(progress(0, 0), 0);
        assert_eq!(progress(256, 0), 256);
        assert_eq!(progress(4352, 256), 4096);
        assert_eq!(progress(2, 0u16.wrapping_sub(2)), 4);
    }
}
