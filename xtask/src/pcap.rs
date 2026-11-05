//! pcap — a minimal classic-libpcap + ARP classifier for boot-test gate
//! evidence (D-06).  qemu's `-object filter-dump` writes classic pcap: a
//! 24-byte global header (magic 0xa1b2c3d4 in the writer's byte order —
//! the bytes `d4 c3 b2 a1` on the little-endian gate hosts — version 2.4,
//! snaplen, linktype 1 = Ethernet), then 16-byte record headers
//! {ts_sec, ts_usec, caplen, len} plus frame bytes.  The parser reads gate
//! evidence, so every read is bounds-checked: truncation or a bad magic is
//! a loud Err, never a panic and never a partial frame (T-07-01).
//!
//! Zero new crate dependencies — std-only byte slices, the
//! contains_marker/count_marker_lines discipline (main.rs).
//!
//! The net-gate assertion suite (07-05, D-06/D-08): `assert_arp_pingpong`
//! (arp mode: N requests <-> N replies, stats/ifstats reconciliation) and
//! `assert_blast` (TX-bulk: pcap count == `netblast: sent` == stats out,
//! zero RX).  The serial-side extraction helpers — the guest-MAC bind
//! line, the stats `in:`/`out:` counters, the per-queue ifstats
//! `vq<q>: avail=<a> used=<u> lastused=<l> nintr=<x> nnote=<m>` lines —
//! live in this module (named here per the plan's placement rule), so the
//! whole assertion half is one self-contained unit over the pcap frames
//! and the serial capture.

/// One classified ARP frame (ethertype 0x0806) with the fields the net
/// gates assert on.  Multi-byte wire fields are big-endian (network order)
/// regardless of the capture file's byte order.
#[derive(Debug)]
pub struct ArpFrame {
    /// Ethernet source MAC (the frame's sender).
    pub src_mac: [u8; 6],
    /// ARP sender hardware address.
    pub sha: [u8; 6],
    /// ARP sender protocol address.
    pub spa: [u8; 4],
    /// ARP target hardware address.
    pub tha: [u8; 6],
    /// ARP target protocol address.
    pub tpa: [u8; 4],
    /// ARP operation: 1 = request, 2 = reply.
    pub oper: u16,
    /// Captured frame length in bytes (the record's caplen).
    pub frame_len: usize,
}

/// Parse a classic-pcap capture, returning every ARP frame it carries
/// (records of other ethertypes are frames, not ARP — they are counted by
/// frame_count and skipped here).  Errs loudly on a bad magic, a
/// non-Ethernet linktype, or a truncated record.
pub fn parse_pcap(bytes: &[u8]) -> Result<Vec<ArpFrame>, String> {
    let mut frames = Vec::new();
    for_each_record(bytes, &mut |frame| {
        if let Some(arp) = classify_arp(frame) {
            frames.push(arp);
        }
    })?;
    Ok(frames)
}

/// Count ALL records in the capture, regardless of ethertype.
pub fn frame_count(bytes: &[u8]) -> Result<usize, String> {
    let mut n = 0usize;
    for_each_record(bytes, &mut |_| n += 1)?;
    Ok(n)
}

/// ARP requests (oper 1) among parsed frames.
pub fn request_count(frames: &[ArpFrame]) -> usize {
    frames.iter().filter(|f| f.oper == 1).count()
}

/// ARP replies (oper 2) among parsed frames.
pub fn reply_count(frames: &[ArpFrame]) -> usize {
    frames.iter().filter(|f| f.oper == 2).count()
}

/// The arp-mode (ping-pong) gate assertion, NETDRV-03: N guest ARP
/// requests (oper=1, sha = the guest MAC parsed from the capture's
/// `#l0: virtio:` bind line — never a hardcoded constant, spa 10.0.2.15,
/// tpa 10.0.2.2, len 60) elicit exactly N slirp replies (oper=2,
/// spa 10.0.2.2, tpa 10.0.2.15, tha = the guest MAC), with N derived from
/// the data (requests == replies == stats in == stats out, N >= 1), the
/// pcap's total frame count == 2N (idle slirp emits nothing else), and
/// the ifstats per-queue reconciliation (nintr in [1, frames] on a queue
/// that moved frames; used == lastused at print time).  Returns N.
/// Every failure names the mismatched field with got/want values.
pub fn assert_arp_pingpong(
    frames: &[ArpFrame],
    total_frames: usize,
    capture: &[u8],
) -> Result<usize, String> {
    // Placeholder body so the Net profile's finish() call sites (07-05
    // task 1) compile; the real assertion lands with this plan's task 2
    // TDD cycle (its RED fixtures fail against this Err; GREEN replaces
    // the body).  Never reached on the live red path — the sweep's
    // fast-fail fires before finish().
    let _ = (frames, total_frames, capture);
    Err("assert_arp_pingpong: assertion suite lands in 07-05 task 2".to_string())
}

/// The blast-mode (TX-bulk) gate assertion, NETDRV-04's load leg: the
/// pcap's frames == the parsed `netblast: sent <n>` count == stats out
/// (fresh boot: stats in == 0), every frame an ARP request (oper=1,
/// sha = the guest MAC, tpa 10.0.2.99, len 60) with zero reply frames,
/// NET-DONE present, and the same ifstats reconciliation.  Returns the
/// blast count.  A wedge manifests as a deadline miss, a count mismatch,
/// or a negative-scan hit — never silence.
pub fn assert_blast(
    frames: &[ArpFrame],
    total_frames: usize,
    capture: &[u8],
) -> Result<usize, String> {
    // Placeholder body — same task-2 TDD contract as
    // assert_arp_pingpong above.
    let _ = (frames, total_frames, capture);
    Err("assert_blast: assertion suite lands in 07-05 task 2".to_string())
}

/// Validate the 24-byte global header, returning the capture's byte order
/// (true = little-endian).  The magic's byte sequence names the order (the
/// classic-pcap reader convention): qemu's filter-dump writes host-endian,
/// so the gate hosts' captures are the d4 c3 b2 a1 shape, but a
/// big-endian a1 b2 c3 d4 capture is equally valid and parses identically.
fn global_header(bytes: &[u8]) -> Result<bool, String> {
    if bytes.len() < 24 {
        return Err(format!(
            "pcap: buffer is {} bytes, shorter than the 24-byte global header",
            bytes.len()
        ));
    }
    let little_endian = match &bytes[0..4] {
        [0xa1, 0xb2, 0xc3, 0xd4] => false,
        [0xd4, 0xc3, 0xb2, 0xa1] => true,
        magic => {
            return Err(format!(
                "pcap: bad magic {:02x}{:02x}{:02x}{:02x} — not a classic-pcap capture \
                 (0xa1b2c3d4 in either byte order)",
                magic[0], magic[1], magic[2], magic[3]
            ));
        }
    };
    let linktype = read_u32(bytes, 20, little_endian);
    if linktype != 1 {
        return Err(format!("pcap: linktype {linktype} is not Ethernet (1)"));
    }
    Ok(little_endian)
}

/// A capture-endianness u32.  Callers bounds-check the four-byte window
/// before calling, so the fixed-size conversion is infallible.
fn read_u32(bytes: &[u8], off: usize, le: bool) -> u32 {
    let b: [u8; 4] = bytes[off..off + 4].try_into().expect("caller bounds-checks");
    if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) }
}

/// Walk every record's captured frame bytes.  Shared by parse_pcap and
/// frame_count so the magic/truncation strictness lives in exactly one
/// place.
fn for_each_record(bytes: &[u8], f: &mut dyn FnMut(&[u8])) -> Result<(), String> {
    let le = global_header(bytes)?;
    let mut off = 24usize;
    while off < bytes.len() {
        let remaining = bytes.len() - off;
        if remaining < 16 {
            return Err(format!(
                "pcap: truncated record header at offset {off}: {remaining} of 16 bytes"
            ));
        }
        let caplen = read_u32(bytes, off + 8, le) as usize;
        let frame_start = off + 16;
        let available = bytes.len() - frame_start;
        if caplen > available {
            return Err(format!(
                "pcap: truncated record at offset {off}: caplen {caplen} exceeds the \
                 {available} remaining bytes"
            ));
        }
        f(&bytes[frame_start..frame_start + caplen]);
        off = frame_start + caplen;
    }
    Ok(())
}

/// Classify one Ethernet frame as ARP (ethertype 0x0806), extracting the
/// fields the gates assert on; None for any other ethertype or a runt
/// frame (shorter than the 14-byte Ethernet header + 28-byte ARP body —
/// counted as a frame, never parsed).
fn classify_arp(frame: &[u8]) -> Option<ArpFrame> {
    if frame.len() < 42 || u16::from_be_bytes([frame[12], frame[13]]) != 0x0806 {
        return None;
    }
    // The length check above makes every fixed-size conversion below
    // infallible; wire fields are big-endian (network order) regardless of
    // the capture's byte order.
    Some(ArpFrame {
        src_mac: frame[6..12].try_into().expect("len >= 42"),
        oper: u16::from_be_bytes([frame[20], frame[21]]),
        sha: frame[22..28].try_into().expect("len >= 42"),
        spa: frame[28..32].try_into().expect("len >= 42"),
        tha: frame[32..38].try_into().expect("len >= 42"),
        tpa: frame[38..42].try_into().expect("len >= 42"),
        frame_len: frame.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The qemu default guest MAC (slirp user networking).
    const GUEST_MAC: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
    /// slirp's emulated gateway: 10.0.2.2, answered from 52:55:0a:00:02:02
    /// (slirp's 52:55:IP:IP:IP:IP convention).
    const GATEWAY_MAC: [u8; 6] = [0x52, 0x55, 0x0a, 0x00, 0x02, 0x02];
    const GUEST_IP: [u8; 4] = [10, 0, 2, 15];
    const GATEWAY_IP: [u8; 4] = [10, 0, 2, 2];

    /// 24-byte little-endian classic-pcap global header — qemu's
    /// filter-dump writes host-endian (the LE magic bytes d4 c3 b2 a1 on
    /// every gate host), version 2.4, snaplen 65536, linktype 1 (Ethernet).
    fn global_header() -> Vec<u8> {
        let mut v = vec![0xd4, 0xc3, 0xb2, 0xa1, 0x02, 0x00, 0x04, 0x00];
        v.extend_from_slice(&[0u8; 8]); // thiszone + sigfigs
        v.extend_from_slice(&65536u32.to_le_bytes()); // snaplen
        v.extend_from_slice(&1u32.to_le_bytes()); // linktype 1 = Ethernet
        v
    }

    /// A 16-byte record header (le32 ts_sec, ts_usec, caplen, len) + frame.
    fn record(frame: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&1u32.to_le_bytes()); // ts_sec (arbitrary)
        v.extend_from_slice(&0u32.to_le_bytes()); // ts_usec
        v.extend_from_slice(&(frame.len() as u32).to_le_bytes()); // caplen
        v.extend_from_slice(&(frame.len() as u32).to_le_bytes()); // len
        v.extend_from_slice(frame);
        v
    }

    /// 60-byte ARP request (the ETHERMINTU-padded D-07 erratum shape):
    /// broadcast dst, guest src, oper 1, spa 10.0.2.15, tpa 10.0.2.2.
    fn arp_request() -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&[0xff; 6]); // dst: broadcast
        f.extend_from_slice(&GUEST_MAC); // src
        f.extend_from_slice(&[0x08, 0x06]); // ethertype: ARP
        f.extend_from_slice(&[0x00, 0x01]); // htype: Ethernet
        f.extend_from_slice(&[0x08, 0x00]); // ptype: IPv4
        f.extend_from_slice(&[6, 4]); // hlen, plen
        f.extend_from_slice(&[0x00, 0x01]); // oper: request
        f.extend_from_slice(&GUEST_MAC); // sha
        f.extend_from_slice(&GUEST_IP); // spa
        f.extend_from_slice(&[0x00; 6]); // tha
        f.extend_from_slice(&GATEWAY_IP); // tpa
        f.extend_from_slice(&[0x00; 18]); // ETHERMINTU pad
        f
    }

    /// 60-byte ARP reply from the slirp gateway (oper 2).
    fn arp_reply() -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&GUEST_MAC); // dst: the guest
        f.extend_from_slice(&GATEWAY_MAC); // src: slirp's emulated host
        f.extend_from_slice(&[0x08, 0x06]);
        f.extend_from_slice(&[0x00, 0x01]);
        f.extend_from_slice(&[0x08, 0x00]);
        f.extend_from_slice(&[6, 4]);
        f.extend_from_slice(&[0x00, 0x02]); // oper: reply
        f.extend_from_slice(&GATEWAY_MAC); // sha
        f.extend_from_slice(&GATEWAY_IP); // spa
        f.extend_from_slice(&GUEST_MAC); // tha
        f.extend_from_slice(&GUEST_IP); // tpa
        f.extend_from_slice(&[0x00; 18]);
        f
    }

    /// Test 1 (golden request): one synthetic capture carrying one 60-byte
    /// ARP request parses to exactly one frame with every field extracted.
    #[test]
    fn golden_arp_request_parses_all_fields() {
        let mut buf = global_header();
        buf.extend_from_slice(&record(&arp_request()));
        let frames = parse_pcap(&buf).expect("a valid capture parses");
        assert_eq!(frames.len(), 1);
        let f = &frames[0];
        assert_eq!(f.oper, 1);
        assert_eq!(f.src_mac, GUEST_MAC);
        assert_eq!(f.sha, GUEST_MAC);
        assert_eq!(f.spa, GUEST_IP);
        assert_eq!(f.tha, [0; 6]);
        assert_eq!(f.tpa, GATEWAY_IP);
        assert_eq!(f.frame_len, 60);
    }

    /// Test 2 (golden reply): a 60-byte ARP reply classifies with its own
    /// field set (oper 2, gateway sha/spa, guest tha/tpa).
    #[test]
    fn golden_arp_reply_parses_all_fields() {
        let mut buf = global_header();
        buf.extend_from_slice(&record(&arp_reply()));
        let frames = parse_pcap(&buf).expect("a valid capture parses");
        assert_eq!(frames.len(), 1);
        let f = &frames[0];
        assert_eq!(f.oper, 2);
        assert_eq!(f.src_mac, GATEWAY_MAC);
        assert_eq!(f.sha, GATEWAY_MAC);
        assert_eq!(f.spa, GATEWAY_IP);
        assert_eq!(f.tha, GUEST_MAC);
        assert_eq!(f.tpa, GUEST_IP);
        assert_eq!(f.frame_len, 60);
    }

    /// Test 3 (non-ARP filtered): an ethertype-0x0800 record is skipped by
    /// the ARP classifier but still counted as a frame.
    #[test]
    fn non_arp_frame_is_counted_but_not_classified() {
        let mut frame = arp_request();
        frame[12] = 0x08;
        frame[13] = 0x00; // ethertype: IPv4
        let mut buf = global_header();
        buf.extend_from_slice(&record(&frame));
        let frames = parse_pcap(&buf).expect("a valid capture parses");
        assert!(frames.is_empty(), "non-ARP frames are skipped by the ARP classifier");
        assert_eq!(frame_count(&buf).expect("a valid capture counts"), 1);
    }

    /// Test 4 (truncation safety): a buffer cut mid-record (caplen longer
    /// than the remaining bytes) is a loud parse error naming the
    /// truncation — never a panic, never a partial frame.
    #[test]
    fn truncated_record_errors_loudly() {
        let mut buf = global_header();
        let full = record(&arp_request());
        buf.extend_from_slice(&full[..16 + 30]); // caplen says 60, 30 present
        let res = parse_pcap(&buf);
        assert!(res.is_err(), "a truncated record must error, never a partial frame");
        let e = res.unwrap_err();
        assert!(e.contains("truncat"), "the error names the truncation: {e}");
    }

    /// Test 5 (bad magic): a buffer whose first four bytes are not the
    /// classic-pcap magic (in either byte order) is a loud error naming
    /// the magic mismatch.
    #[test]
    fn bad_magic_errors_loudly() {
        let mut buf = global_header();
        buf[0..4].copy_from_slice(&[0x00, 0x01, 0x02, 0x03]);
        let res = parse_pcap(&buf);
        assert!(res.is_err(), "a bad magic must error");
        let e = res.unwrap_err();
        assert!(e.contains("magic"), "the error names the magic mismatch: {e}");
    }

    /// Test 6 (count helpers): a fixture of two requests + one reply yields
    /// request_count 2, reply_count 1 (and frame_count counts all three).
    #[test]
    fn request_reply_counts_over_mixed_fixture() {
        let mut buf = global_header();
        buf.extend_from_slice(&record(&arp_request()));
        buf.extend_from_slice(&record(&arp_request()));
        buf.extend_from_slice(&record(&arp_reply()));
        let frames = parse_pcap(&buf).expect("a valid capture parses");
        assert_eq!(frames.len(), 3);
        assert_eq!(request_count(&frames), 2);
        assert_eq!(reply_count(&frames), 1);
        assert_eq!(frame_count(&buf).expect("a valid capture counts"), 3);
    }

    /// The plan text's magic spelling made flesh: a big-endian capture
    /// (the a1 b2 c3 d4 byte order) is equally valid classic pcap — the
    /// magic's byte sequence, not the host, names the file's byte order.
    #[test]
    fn big_endian_magic_accepted() {
        let frame = arp_request();
        let mut buf = vec![0xa1, 0xb2, 0xc3, 0xd4, 0x00, 0x02, 0x00, 0x04];
        buf.extend_from_slice(&[0u8; 8]); // thiszone + sigfigs
        buf.extend_from_slice(&65536u32.to_be_bytes()); // snaplen
        buf.extend_from_slice(&1u32.to_be_bytes()); // linktype 1 = Ethernet
        buf.extend_from_slice(&1u32.to_be_bytes()); // ts_sec
        buf.extend_from_slice(&0u32.to_be_bytes()); // ts_usec
        buf.extend_from_slice(&(frame.len() as u32).to_be_bytes()); // caplen
        buf.extend_from_slice(&(frame.len() as u32).to_be_bytes()); // len
        buf.extend_from_slice(&frame);
        let frames = parse_pcap(&buf).expect("a big-endian capture parses");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].oper, 1);
        assert_eq!(frames[0].tpa, GATEWAY_IP);
    }

    // ---- 07-05 task 2: the net-gate assertion suite (D-06/D-08) -------
    // Fixtures below build the SERIAL capture the sweep produces (bind
    // line, stats block, ifstats lines, markers) and the pcap frame sets
    // each mode asserts over.  The guest MAC appears ONLY in fixtures
    // (the bind line's tail bytes) — the assertion path parses it out,
    // never reads a constant.

    /// The guest-MAC bind line the driver prints at attach (RESEARCH
    /// Q11's target shape) — kernel-printed, so CRLF-cooked on the wire.
    const BIND_LINE: &str = "#l0: virtio: 1000Mbps: 52:54:00:12:34:56\r\n";

    /// A blast-mode TX frame: netblast's static 60-byte ARP request with
    /// tpa 10.0.2.99 (no slirp responder — pure TX, zero RX noise).
    fn blast_request() -> Vec<u8> {
        let mut f = arp_request();
        f[38..42].copy_from_slice(&[10, 0, 2, 99]); // tpa
        f
    }

    /// The netif.c stats block (in:/out: are the gate's counters), as the
    /// sweep's cat prints it — userland writes are byte-exact (bare \n).
    fn stats_block(in_n: u64, out_n: u64) -> String {
        format!(
            "in: {in_n}\nlink: 1\nout: {out_n}\ncrc errs: 0\noverflows: 0\n\
             soft overflows: 0\nframing errs: 0\nbuffer errs: 0\noutput errs: 0\n\
             prom: 0\nmbps: 1000\naddr: 525400123456\n"
        )
    }

    /// One ifstats per-queue line (the 07-07 driver hook's shape):
    /// `vq<q>: avail=<a> used=<u> lastused=<l> nintr=<x> nnote=<m>`.
    fn ifstats_line(q: u32, used: u64, lastused: u64, nintr: u64) -> String {
        format!("vq{q}: avail={used} used={used} lastused={lastused} nintr={nintr} nnote=0\n")
    }

    /// The arp-mode golden capture: bind line + stats(N,N) + both queues
    /// reconciled (nintr in [1,N], used == lastused) + NET-DONE.
    fn arp_capture(n: u64) -> Vec<u8> {
        let mut s = String::new();
        s.push_str(BIND_LINE);
        s.push_str(&stats_block(n, n));
        s.push_str(&ifstats_line(0, n, n, 2));
        s.push_str(&ifstats_line(1, n, n, 2));
        s.push_str("netsweep: NET-DONE\r\n");
        s.into_bytes()
    }

    /// The blast-mode golden capture: bind line + netblast's sent line +
    /// stats (in 0, out N) + queues (RX unmoved: nintr=0; TX reconciled)
    /// + NET-DONE.
    fn blast_capture(n: u64) -> Vec<u8> {
        let mut s = String::new();
        s.push_str(BIND_LINE);
        s.push_str(&format!("netblast: sent {n} frames\r\n"));
        s.push_str(&stats_block(0, n));
        s.push_str(&ifstats_line(0, 0, 0, 0));
        s.push_str(&ifstats_line(1, n, n, 7));
        s.push_str("netsweep: NET-DONE\r\n");
        s.into_bytes()
    }

    /// A pcap carrying n requests + n replies (the arp ping-pong shape).
    fn pingpong_pcap(n: usize) -> Vec<u8> {
        let mut buf = global_header();
        for _ in 0..n {
            buf.extend_from_slice(&record(&arp_request()));
        }
        for _ in 0..n {
            buf.extend_from_slice(&record(&arp_reply()));
        }
        buf
    }

    /// A pcap carrying n blast requests (pure TX).
    fn blast_pcap(n: usize) -> Vec<u8> {
        let mut buf = global_header();
        for _ in 0..n {
            buf.extend_from_slice(&record(&blast_request()));
        }
        buf
    }

    /// Golden arp mode: 4 requests + 4 replies + the reconciled capture
    /// passes, with N=4 derived from the data (never a constant).
    #[test]
    fn arp_pingpong_golden() {
        let buf = pingpong_pcap(4);
        let frames = parse_pcap(&buf).expect("a valid capture parses");
        let total = frame_count(&buf).expect("a valid capture counts");
        let n = assert_arp_pingpong(&frames, total, &arp_capture(4)).expect("golden arp passes");
        assert_eq!(n, 4, "N is derived from the capture data");
    }

    /// Negative: 4 requests but 3 replies fails naming the count mismatch.
    #[test]
    fn arp_count_mismatch_fails() {
        let mut buf = global_header();
        for _ in 0..4 {
            buf.extend_from_slice(&record(&arp_request()));
        }
        for _ in 0..3 {
            buf.extend_from_slice(&record(&arp_reply()));
        }
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let e = assert_arp_pingpong(&frames, total, &arp_capture(4))
            .expect_err("4 requests + 3 replies must fail");
        assert!(e.contains("count mismatch"), "names the count mismatch: {e}");
    }

    /// Negative: a request with tpa 10.0.2.3 (slirp's DNS address, not
    /// the gateway) fails naming the field with the got value.
    #[test]
    fn arp_wrong_field_fails() {
        let mut bad = arp_request();
        bad[38..42].copy_from_slice(&[10, 0, 2, 3]); // tpa: not the gateway
        let mut buf = global_header();
        buf.extend_from_slice(&record(&bad));
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let e = assert_arp_pingpong(&frames, total, &arp_capture(1))
            .expect_err("a wrong-tpa request must fail");
        assert!(e.contains("tpa"), "names the field: {e}");
        assert!(e.contains("10.0.2.3"), "the got value is present: {e}");
    }

    /// Negative: an extra non-ARP frame (total != 2N) fails naming the
    /// 2N invariant — idle slirp emits nothing else.
    #[test]
    fn arp_extra_frame_fails() {
        let mut buf = pingpong_pcap(4);
        let mut ipv4 = arp_request();
        ipv4[12] = 0x08;
        ipv4[13] = 0x00; // ethertype IPv4: a frame, not ARP
        buf.extend_from_slice(&record(&ipv4));
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let e = assert_arp_pingpong(&frames, total, &arp_capture(4))
            .expect_err("an extra frame must fail");
        assert!(e.contains("2N"), "names the 2N invariant: {e}");
    }

    /// Negative: no `#l0: virtio:` bind line in the capture — the guest
    /// MAC source is gone — fails naming it.
    #[test]
    fn arp_missing_bind_line_fails() {
        let buf = pingpong_pcap(1);
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let capture = b"netsweep: NET-DONE\r\n".to_vec();
        let e = assert_arp_pingpong(&frames, total, &capture)
            .expect_err("a capture with no bind line must fail");
        assert!(e.contains("#l0: virtio:"), "names the bind line: {e}");
    }

    /// Negative: stats counters absent from the capture fail loudly
    /// (T-07-07: absence or mismatch, never a silent pass).
    #[test]
    fn arp_missing_stats_fails() {
        let buf = pingpong_pcap(1);
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let mut capture = BIND_LINE.to_string();
        capture.push_str(&ifstats_line(0, 1, 1, 1));
        capture.push_str(&ifstats_line(1, 1, 1, 1));
        let e = assert_arp_pingpong(&frames, total, capture.as_bytes())
            .expect_err("a capture with no stats block must fail");
        assert!(e.contains("in:"), "names the missing stats key: {e}");
    }

    /// Golden blast mode: 256 TX frames + the matching capture passes,
    /// returning the blast count.
    #[test]
    fn blast_golden() {
        let buf = blast_pcap(256);
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let n = assert_blast(&frames, total, &blast_capture(256)).expect("golden blast passes");
        assert_eq!(n, 256);
    }

    /// Negative: netblast's sent count disagrees with the pcap's frame
    /// count — fails naming the mismatch.
    #[test]
    fn blast_sent_count_mismatch_fails() {
        let buf = blast_pcap(256);
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let e = assert_blast(&frames, total, &blast_capture(255))
            .expect_err("sent 255 vs 256 frames must fail");
        assert!(e.contains("sent"), "names the sent-count mismatch: {e}");
    }

    /// Negative: any oper=2 frame in a blast capture fails — the TX-bulk
    /// leg carries zero replies.
    #[test]
    fn blast_reply_present_fails() {
        let mut buf = blast_pcap(4);
        buf.extend_from_slice(&record(&arp_reply()));
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let e = assert_blast(&frames, total, &blast_capture(4))
            .expect_err("a reply frame in blast mode must fail");
        assert!(e.contains("repl"), "names the zero-reply invariant: {e}");
    }

    /// Negative: stats in != 0 on a fresh blast boot fails (no RX traffic
    /// may arrive — tpa 10.0.2.99 has no responder).
    #[test]
    fn blast_rx_noise_fails() {
        let buf = blast_pcap(2);
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let mut capture = String::new();
        capture.push_str(BIND_LINE);
        capture.push_str("netblast: sent 2 frames\r\n");
        capture.push_str(&stats_block(1, 2)); // in: 1 — RX noise
        capture.push_str(&ifstats_line(0, 0, 0, 0));
        capture.push_str(&ifstats_line(1, 2, 2, 1));
        capture.push_str("netsweep: NET-DONE\r\n");
        let e = assert_blast(&frames, total, capture.as_bytes())
            .expect_err("stats in != 0 must fail");
        assert!(e.contains("in:"), "names the in counter: {e}");
    }

    /// Negative: nintr == 0 on a queue that moved frames fails — an
    /// interrupt never fired for work the queue provably did.
    #[test]
    fn ifstats_nintr_zero_fails() {
        let buf = pingpong_pcap(4);
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let mut capture = String::new();
        capture.push_str(BIND_LINE);
        capture.push_str(&stats_block(4, 4));
        capture.push_str(&ifstats_line(0, 4, 4, 2));
        capture.push_str(&ifstats_line(1, 4, 4, 0)); // TX moved 4, nintr 0
        capture.push_str("netsweep: NET-DONE\r\n");
        let e = assert_arp_pingpong(&frames, total, capture.as_bytes())
            .expect_err("nintr=0 on a moved queue must fail");
        assert!(e.contains("nintr"), "names nintr: {e}");
    }

    /// Negative: used != lastused at print time fails naming the
    /// undrained ring (stuck entries).
    #[test]
    fn ifstats_undrained_ring_fails() {
        let buf = pingpong_pcap(4);
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let mut capture = String::new();
        capture.push_str(BIND_LINE);
        capture.push_str(&stats_block(4, 4));
        capture.push_str(&ifstats_line(0, 4, 3, 2)); // used 4, lastused 3
        capture.push_str(&ifstats_line(1, 4, 4, 2));
        capture.push_str("netsweep: NET-DONE\r\n");
        let e = assert_arp_pingpong(&frames, total, capture.as_bytes())
            .expect_err("used != lastused must fail");
        assert!(e.contains("undrained"), "names the undrained ring: {e}");
    }

    /// The red-path live capture's shape: a 24-byte-header-only pcap (an
    /// idle NIC emits nothing) parses to zero frames without error.
    #[test]
    fn empty_capture_parses_zero_frames() {
        let buf = global_header();
        let frames = parse_pcap(&buf).expect("the header-only capture parses");
        assert!(frames.is_empty());
        assert_eq!(frame_count(&buf).expect("counts"), 0);
    }
}
