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
    let _ = bytes;
    Ok(Vec::new()) // RED stub — the GREEN commit implements the real parse
}

/// Count ALL records in the capture, regardless of ethertype.
pub fn frame_count(bytes: &[u8]) -> Result<usize, String> {
    let _ = bytes;
    Ok(0) // RED stub
}

/// ARP requests (oper 1) among parsed frames.
pub fn request_count(frames: &[ArpFrame]) -> usize {
    let _ = frames;
    0 // RED stub
}

/// ARP replies (oper 2) among parsed frames.
pub fn reply_count(frames: &[ArpFrame]) -> usize {
    let _ = frames;
    0 // RED stub
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
