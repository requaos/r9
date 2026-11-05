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
}
