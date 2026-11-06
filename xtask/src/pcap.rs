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

// ---- The net-gate assertion machinery (07-05 task 2, D-06/D-08) -------
//
// The sweep's evidence arrives as the serial capture: the driver's
// `#l0: virtio: ...` bind line (the guest-MAC source — the assertions
// never read a hardcoded MAC), the netif.c stats block (`in: <n>` /
// `out: <n>` counters), the per-queue ifstats lines
// (`vq<q>: avail=<a> used=<u> lastused=<l> nintr=<x> nnote=<m>`), and the
// sweep's own markers.  Byte-slice scanning in the count_marker_lines
// style: split on \n, trim one trailing \r (kernel prints are CRLF-cooked,
// the sweep's cat output is byte-exact \n), key on line prefixes.  Every
// failure names the mismatched field with got/want values — a gate
// failure is a debugging session's head start, never a bare mismatch
// (T-07-07: absence or malformation is a loud Err, never a silent pass).

/// slirp's fixed addressing (RESEARCH Q9, the libslirp defaults): the
/// guest's DHCP address and the gateway the ping-pong ARPs target.
const SLIRP_GUEST_IP: [u8; 4] = [10, 0, 2, 15];
const SLIRP_GATEWAY_IP: [u8; 4] = [10, 0, 2, 2];
/// The blast mode's unreachable target (no slirp responder — pure TX).
const BLAST_TARGET_IP: [u8; 4] = [10, 0, 2, 99];
/// devether's write path enforces ETHERMINTU: every frame on the wire is
/// exactly 60 bytes.
const ETHERMINTU: usize = 60;

// ---- The D-11 addressing seam reader (08-06) ---------------------------
//
// The netip static gate's assertions derive their addresses from
// $R9_HARVEY_DIR/build/aarch64/netaddr.rc (the rc KEY=VALUE seam file the
// fixture sources in-guest) — NO 10.0.2.x literal in the 08-06
// assertions (the phase's transparency prohibition: the sweep, the baked
// fragments, and these assertions all derive from the seam).

/// A dotted-quad parse ("10.0.2.15" → [10,0,2,15]).
fn parse_ipv4_dotted(s: &str) -> Option<[u8; 4]> {
    let mut out = [0u8; 4];
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    for (i, p) in parts.iter().enumerate() {
        out[i] = p.parse().ok()?;
    }
    Some(out)
}

/// The slirp leg's (guest, gateway) addresses, parsed from the seam
/// file's `slirp_guestv4=` / `slirp_gwv4=` lines.  An unreadable file or
/// an absent/malformed key is a loud error naming the file/key (the
/// 07-01 harvey:-channel precedent — a pre-spawn harness bug, never a
/// silent default).
pub fn netaddr_slirp() -> Result<([u8; 4], [u8; 4]), String> {
    let path = crate::uregcheck::harvey_dir()
        .map_err(|e| format!("{e}"))?
        .join("build/aarch64/netaddr.rc");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("net gate: read {}: {e}", path.display()))?;
    let get = |key: &str| -> Result<[u8; 4], String> {
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            if let Some(v) = line.strip_prefix(key).and_then(|r| r.strip_prefix('=')) {
                return parse_ipv4_dotted(v.trim()).ok_or_else(|| {
                    format!("net gate: {} key {key} is not a dotted quad: {v:?}", path.display())
                });
            }
        }
        Err(format!("net gate: {} lacks the {key}= line", path.display()))
    };
    Ok((get("slirp_guestv4")?, get("slirp_gwv4")?))
}

// ---- The ICMPv4 classifier (08-06 — the netip static gate's ping leg) --

/// One classified ICMPv4 frame (ethertype 0x0800, IP proto 1) with the
/// fields the netip static gate asserts on.  Wire fields are big-endian
/// regardless of the capture's byte order.
#[derive(Debug)]
pub struct Icmpv4Frame {
    /// Ethernet source MAC (the frame's sender).
    pub src_mac: [u8; 6],
    /// IP source / destination.
    pub src: [u8; 4],
    pub dst: [u8; 4],
    /// The ICMP type byte (8 = echo request, 0 = echo reply).
    pub icmp_type: u8,
    /// Captured frame length in bytes.
    pub frame_len: usize,
}

/// Parse every ICMPv4 frame in the capture (other records are counted by
/// frame_count and skipped here).  Same loud-Err discipline as
/// parse_pcap.
pub fn parse_pcap_icmpv4(bytes: &[u8]) -> Result<Vec<Icmpv4Frame>, String> {
    let mut frames = Vec::new();
    for_each_record(bytes, &mut |frame| {
        if let Some(f) = classify_icmpv4(frame) {
            frames.push(f);
        }
    })?;
    Ok(frames)
}

/// Classify one Ethernet frame as ICMPv4 (ethertype 0x0800, IP proto 1),
/// extracting the gate's fields; None for any other shape or a runt
/// frame (shorter than eth + minimal-IPv4 + the ICMP type byte).
fn classify_icmpv4(frame: &[u8]) -> Option<Icmpv4Frame> {
    if frame.len() < 14 + 20 + 1 || u16::from_be_bytes([frame[12], frame[13]]) != 0x0800 {
        return None;
    }
    let ip = &frame[14..];
    let ihl = (ip[0] & 0x0f) as usize * 4;
    if ihl < 20 || frame.len() < 14 + ihl + 1 || ip[9] != 1 {
        return None;
    }
    Some(Icmpv4Frame {
        src_mac: frame[6..12].try_into().expect("len >= 35"),
        src: ip[12..16].try_into().expect("len checked"),
        dst: ip[16..20].try_into().expect("len checked"),
        icmp_type: frame[14 + ihl],
        frame_len: frame.len(),
    })
}

/// assert_gratuitous_arp (NETIP-04's wire proof of the donor's
/// areg=sendgarp, contract §10): an ARP REQUEST whose sender protocol
/// address equals the guest's SEAM address appears in this run's capture
/// — the who-has-SELF shape (spa == tpa == guest) — sourced from the
/// guest MAC (the capture's bind line, never a constant).  devip emits
/// the gratuitous ARP on ipconfig's address add; smoltcp never does.
/// `guest` is the seam's slirp_guestv4 (the caller resolves the seam —
/// netaddr_slirp — so this assertion stays host-testable).
pub fn assert_gratuitous_arp(frames: &[ArpFrame], guest: [u8; 4], capture: &[u8]) -> Result<(), String> {
    let mac = guest_mac(capture)?;
    for f in frames {
        if f.oper == 1 && f.spa == guest && f.tpa == guest {
            if f.sha != mac {
                return Err(format!(
                    "net gate (static): the gratuitous ARP's sha got {} want {} (the guest MAC)",
                    mac_str(&f.sha),
                    mac_str(&mac)
                ));
            }
            return Ok(());
        }
    }
    Err(format!(
        "net gate (static): no gratuitous ARP (oper=1, spa == tpa == {}) in this run's capture — \
         the devip sendgarp never emitted on the address add",
        ip_str(&guest)
    ))
}

/// assert_icmp_exchange (NETIP-04's ping wire proof): an ICMP echo
/// REQUEST guest→gateway AND an echo REPLY gateway→guest in this run's
/// capture — the request's ethernet source is the guest MAC (the capture's
/// bind line).  `guest`/`gw` are the seam's slirp_guestv4/slirp_gwv4 (the
/// caller resolves the seam — netaddr_slirp).  Returns the request
/// count.  (The venue label — NOT L2 truth — rides the serial log, not
/// this assertion: the assertion proves the exchange happened, never
/// that the leg is tap-faithful.)
pub fn assert_icmp_exchange(
    frames: &[Icmpv4Frame],
    guest: [u8; 4],
    gw: [u8; 4],
    capture: &[u8],
) -> Result<usize, String> {
    let mac = guest_mac(capture)?;
    let mut reqs = 0;
    let mut reps = 0;
    for f in frames {
        if f.icmp_type == 8 && f.src == guest && f.dst == gw {
            if f.src_mac != mac {
                return Err(format!(
                    "net gate (static): echo request src_mac got {} want {} (the guest MAC)",
                    mac_str(&f.src_mac),
                    mac_str(&mac)
                ));
            }
            reqs += 1;
        }
        if f.icmp_type == 0 && f.src == gw && f.dst == guest {
            reps += 1;
        }
    }
    if reqs == 0 {
        return Err(format!(
            "net gate (static): no ICMP echo request {} → {} in this run's capture",
            ip_str(&guest),
            ip_str(&gw)
        ));
    }
    if reps == 0 {
        return Err(format!(
            "net gate (static): no ICMP echo reply {} → {} in this run's capture ({} requests saw \
             no answer)",
            ip_str(&gw),
            ip_str(&guest),
            reqs
        ));
    }
    Ok(reqs)
}

/// The capture as lines, CRLF-tolerant.
fn capture_lines(capture: &[u8]) -> impl Iterator<Item = &[u8]> {
    capture.split(|b| *b == b'\n').map(|line| match line.last() {
        Some(b'\r') => &line[..line.len() - 1],
        _ => line,
    })
}

fn mac_str(mac: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

fn ip_str(ip: &[u8; 4]) -> String {
    format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}

fn parse_hex_byte(s: &[u8]) -> Option<u8> {
    if s.len() != 2 {
        return None;
    }
    let hi = (s[0] as char).to_digit(16)?;
    let lo = (s[1] as char).to_digit(16)?;
    Some((hi * 16 + lo) as u8)
}

/// The guest MAC, parsed from the driver's `#l0: virtio: ...` bind line
/// (devether.c's print shape; RESEARCH Q11).  The MAC is the line's final
/// 17 bytes (`xx:xx:xx:xx:xx:xx`), whatever the prefix's mbps text says.
fn guest_mac(capture: &[u8]) -> Result<[u8; 6], String> {
    for line in capture_lines(capture) {
        if line.starts_with(b"#l") && line.windows(9).any(|w| w == b": virtio:") {
            if line.len() >= 17 {
                let tail = &line[line.len() - 17..];
                let parts: Vec<&[u8]> = tail.split(|b| *b == b':').collect();
                if parts.len() == 6 {
                    let mut mac = [0u8; 6];
                    let mut ok = true;
                    for (i, p) in parts.iter().enumerate() {
                        match parse_hex_byte(p) {
                            Some(v) => mac[i] = v,
                            None => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if ok {
                        return Ok(mac);
                    }
                }
            }
            return Err(format!(
                "net gate: the `#l0: virtio:` bind line is malformed (no MAC tail): {}",
                String::from_utf8_lossy(line)
            ));
        }
    }
    Err("net gate: no `#l0: virtio:` bind line in the capture — the guest MAC source is gone \
         (devether never bound?)"
        .to_string())
}

/// One stats counter (`in:` / `out:`) from the sweep's stats cat — the
/// netif.c line shape `<key>: <n>` at line start.
fn stat_value(capture: &[u8], key: &str) -> Result<u64, String> {
    let prefix = format!("{key}: ");
    for line in capture_lines(capture) {
        if let Some(rest) = line.strip_prefix(prefix.as_bytes()) {
            let text = String::from_utf8_lossy(rest);
            return text
                .trim()
                .parse::<u64>()
                .map_err(|e| format!("net gate: stats `{prefix}<n>` not numeric ({text:?}): {e}"));
        }
    }
    Err(format!(
        "net gate: no `{prefix}<n>` line in the capture (the sweep's stats cat never ran)"
    ))
}

/// The blaster's sent count from its `netblast: sent <n> frames` marker.
fn netblast_sent(capture: &[u8]) -> Result<u64, String> {
    for line in capture_lines(capture) {
        if let Some(rest) = line.strip_prefix(b"netblast: sent ") {
            let text = String::from_utf8_lossy(rest);
            let num = text.split(' ').next().unwrap_or("");
            return num
                .parse::<u64>()
                .map_err(|e| format!("net gate: `netblast: sent` count not numeric ({num:?}): {e}"));
        }
    }
    Err("net gate: no `netblast: sent <n> frames` line in the capture (netblast never ran?)"
        .to_string())
}

/// One parsed ifstats queue line.  avail/nnote are shape-validated as
/// key=value pairs but carry no plan invariant, so they are not stored.
struct IfStats {
    queue: u64,
    used: u64,
    lastused: u64,
    nintr: u64,
}

/// A `key=<num>` field from an ifstats line's space-separated tokens
/// (prefix-matched at token start, so `used` never matches `lastused`).
fn kv_field(fields: &[u8], key: &str) -> Option<u64> {
    for tok in String::from_utf8_lossy(fields).split_whitespace() {
        if let Some(v) = tok.strip_prefix(key).and_then(|s| s.strip_prefix('=')) {
            return v.parse::<u64>().ok();
        }
    }
    None
}

/// The per-queue ifstats lines: `vq<q>: avail=<a> used=<u> lastused=<l>
/// nintr=<x> nnote=<m>`.  Malformed lines are skipped, not fatal — a
/// capture with NO parsed lines is the loud failure (the reconcile
/// caller), so a stray `vq...` line elsewhere can never silently pass.
fn ifstats_lines(capture: &[u8]) -> Vec<IfStats> {
    let mut out = Vec::new();
    for line in capture_lines(capture) {
        let Some(rest) = line.strip_prefix(b"vq") else { continue };
        let Some(colon) = rest.iter().position(|b| *b == b':') else { continue };
        let Ok(queue) = String::from_utf8_lossy(&rest[..colon]).parse::<u64>() else { continue };
        let fields = &rest[colon + 1..];
        let (Some(used), Some(lastused), Some(nintr)) =
            (kv_field(fields, "used"), kv_field(fields, "lastused"), kv_field(fields, "nintr"))
        else {
            continue;
        };
        out.push(IfStats { queue, used, lastused, nintr });
    }
    out
}

/// The ifstats reconciliation (NETDRV-04's accounting surface): every
/// parsed queue line must show used == lastused at print time (the ring
/// drained — no stuck entries), and a queue that MOVED frames (vq0 = RX
/// rides the stats `in` count, vq1 = TX rides `out` — virtio-net's queue
/// assignment) must show nintr in [1, frames]: an interrupt fired at
/// least once, and never more often than the work it reported.
fn reconcile_ifstats(capture: &[u8], rx_frames: u64, tx_frames: u64) -> Result<(), String> {
    let queues = ifstats_lines(capture);
    if queues.is_empty() {
        return Err("net gate: no ifstats `vq<q>:` lines in the capture (the driver hook never \
                    printed — the sweep's ifstats cat produced nothing)"
            .to_string());
    }
    for q in queues {
        if q.used != q.lastused {
            return Err(format!(
                "net gate: ifstats vq{} undrained ring: used={} != lastused={} at print time \
                 (stuck entries)",
                q.queue, q.used, q.lastused
            ));
        }
        let moved = match q.queue {
            0 => rx_frames,
            1 => tx_frames,
            _ => continue, // a queue beyond RX/TX carries no plan invariant
        };
        if moved > 0 && q.nintr == 0 {
            return Err(format!(
                "net gate: ifstats vq{} moved {moved} frames but nintr=0 — an interrupt never \
                 fired for work the queue provably did",
                q.queue
            ));
        }
        if moved > 0 && q.nintr > moved {
            return Err(format!(
                "net gate: ifstats vq{} nintr={} exceeds the {moved} frames the queue moved \
                 (want nintr in [1, {moved}])",
                q.queue, q.nintr
            ));
        }
    }
    Ok(())
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
pub fn assert_arp_pingpong(
    frames: &[ArpFrame],
    total_frames: usize,
    capture: &[u8],
) -> Result<usize, String> {
    let mac = guest_mac(capture)?;
    let n = request_count(frames);
    let nrep = reply_count(frames);
    if n == 0 {
        return Err(
            "net gate: zero ARP request frames in the pcap (the sweep's dd writes never emitted?)"
                .to_string(),
        );
    }
    for (i, f) in frames.iter().filter(|f| f.oper == 1).enumerate() {
        // devether's write path rewrites the ethernet source to the NIC's
        // own address (devether.c's memmove), so on the wire src_mac ==
        // sha == the guest MAC.
        if f.src_mac != mac {
            return Err(format!(
                "net gate: ARP request #{i}: src_mac got {} want {} (the guest MAC)",
                mac_str(&f.src_mac),
                mac_str(&mac)
            ));
        }
        if f.sha != mac {
            return Err(format!(
                "net gate: ARP request #{i}: sha got {} want {} (the guest MAC)",
                mac_str(&f.sha),
                mac_str(&mac)
            ));
        }
        if f.spa != SLIRP_GUEST_IP {
            return Err(format!(
                "net gate: ARP request #{i}: spa got {} want {}",
                ip_str(&f.spa),
                ip_str(&SLIRP_GUEST_IP)
            ));
        }
        if f.tpa != SLIRP_GATEWAY_IP {
            return Err(format!(
                "net gate: ARP request #{i}: tpa got {} want {} (slirp answers only its own \
                 addresses)",
                ip_str(&f.tpa),
                ip_str(&SLIRP_GATEWAY_IP)
            ));
        }
        if f.frame_len != ETHERMINTU {
            return Err(format!(
                "net gate: ARP request #{i}: len got {} want {ETHERMINTU} (ETHERMINTU)",
                f.frame_len
            ));
        }
    }
    if nrep != n {
        return Err(format!(
            "net gate: ping-pong count mismatch: {n} ARP requests but {nrep} replies (want N==N)"
        ));
    }
    for (i, f) in frames.iter().filter(|f| f.oper == 2).enumerate() {
        if f.spa != SLIRP_GATEWAY_IP {
            return Err(format!(
                "net gate: ARP reply #{i}: spa got {} want {}",
                ip_str(&f.spa),
                ip_str(&SLIRP_GATEWAY_IP)
            ));
        }
        if f.tpa != SLIRP_GUEST_IP {
            return Err(format!(
                "net gate: ARP reply #{i}: tpa got {} want {}",
                ip_str(&f.tpa),
                ip_str(&SLIRP_GUEST_IP)
            ));
        }
        if f.tha != mac {
            return Err(format!(
                "net gate: ARP reply #{i}: tha got {} want {} (the guest MAC)",
                mac_str(&f.tha),
                mac_str(&mac)
            ));
        }
        // The reply's PADDING is slirp's, not our contract — the plan's
        // gate truth pins replies to oper=2 + the address fields, never a
        // length.  Live captures show slirp's replies at 64 bytes (its
        // encap pads past ETHERMINTU); only a RUNT reply (< 60) is a real
        // defect.  (Requests stay pinned at exactly 60 — our own write
        // path's ETHERMINTU enforcement is the thing under test there.)
        if f.frame_len < ETHERMINTU {
            return Err(format!(
                "net gate: ARP reply #{i}: len got {} — runt (want >= {ETHERMINTU})",
                f.frame_len
            ));
        }
    }
    let sin = stat_value(capture, "in")?;
    let sout = stat_value(capture, "out")?;
    if sin != n as u64 {
        return Err(format!("net gate: stats in: got {sin} want {n} (the ping-pong RX count)"));
    }
    if sout != n as u64 {
        return Err(format!("net gate: stats out: got {sout} want {n} (the ping-pong TX count)"));
    }
    if total_frames != 2 * n {
        return Err(format!(
            "net gate: pcap carries {total_frames} frames, want exactly 2N={} ({n} requests + \
             {n} replies) — idle slirp emits nothing else",
            2 * n
        ));
    }
    reconcile_ifstats(capture, n as u64, n as u64)?;
    Ok(n)
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
    let mac = guest_mac(capture)?;
    let sent = netblast_sent(capture)?;
    let sin = stat_value(capture, "in")?;
    let sout = stat_value(capture, "out")?;
    if sin != 0 {
        return Err(format!(
            "net gate: fresh blast boot stats in: got {sin} want 0 (tpa {} has no responder — no \
             RX traffic may arrive)",
            ip_str(&BLAST_TARGET_IP)
        ));
    }
    let nrep = reply_count(frames);
    if nrep != 0 {
        return Err(format!(
            "net gate: blast mode carries {nrep} ARP reply frames, want zero (the TX-bulk leg is \
             pure TX)"
        ));
    }
    for (i, f) in frames.iter().enumerate() {
        if f.oper != 1 {
            return Err(format!(
                "net gate: blast frame #{i}: oper got {} want 1 (requests only)",
                f.oper
            ));
        }
        if f.sha != mac {
            return Err(format!(
                "net gate: blast frame #{i}: sha got {} want {} (the guest MAC)",
                mac_str(&f.sha),
                mac_str(&mac)
            ));
        }
        if f.tpa != BLAST_TARGET_IP {
            return Err(format!(
                "net gate: blast frame #{i}: tpa got {} want {}",
                ip_str(&f.tpa),
                ip_str(&BLAST_TARGET_IP)
            ));
        }
        if f.frame_len != ETHERMINTU {
            return Err(format!(
                "net gate: blast frame #{i}: len got {} want {ETHERMINTU}",
                f.frame_len
            ));
        }
    }
    if frames.len() as u64 != sent {
        return Err(format!(
            "net gate: blast count mismatch: netblast sent {sent} but the pcap carries {} ARP \
             frames",
            frames.len()
        ));
    }
    if total_frames as u64 != sent {
        return Err(format!(
            "net gate: blast count mismatch: netblast sent {sent} but the pcap carries \
             {total_frames} total frames"
        ));
    }
    if sout != sent {
        return Err(format!("net gate: stats out: got {sout} want {sent} (the blast TX count)"));
    }
    if !capture
        .windows(b"netsweep: NET-DONE".len())
        .any(|w| w == b"netsweep: NET-DONE")
    {
        return Err(
            "net gate: netsweep: NET-DONE absent from the capture (the sweep never completed)"
                .to_string(),
        );
    }
    reconcile_ifstats(capture, 0, sent)?;
    Ok(sent as usize)
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

    /// Reply padding is slirp's, not ours (07-07 live-capture finding):
    /// a 64-byte reply passes (slirp's real shape); a 59-byte RUNT reply
    /// fails.  Requests stay pinned at exactly 60 either way.
    #[test]
    fn arp_reply_padding_is_slirps_business() {
        // A one-round capture with nintr=1 (the [1, frames] bound at 1).
        let one_round_capture = || {
            let mut s = String::new();
            s.push_str(BIND_LINE);
            s.push_str(&stats_block(1, 1));
            s.push_str(&ifstats_line(0, 1, 1, 1));
            s.push_str(&ifstats_line(1, 1, 1, 1));
            s.push_str("netsweep: NET-DONE\r\n");
            s.into_bytes()
        };
        let mut buf = global_header();
        buf.extend_from_slice(&record(&arp_request()));
        let mut wide = arp_reply();
        wide.extend_from_slice(&[0u8; 4]); // 64 bytes — the observed slirp shape
        buf.extend_from_slice(&record(&wide));
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let n = assert_arp_pingpong(&frames, total, &one_round_capture())
            .expect("a 64-byte (slirp-shaped) reply passes");
        assert_eq!(n, 1);

        let mut buf = global_header();
        buf.extend_from_slice(&record(&arp_request()));
        let runt = arp_reply()[..59].to_vec();
        buf.extend_from_slice(&record(&runt));
        let frames = parse_pcap(&buf).expect("parses");
        let total = frame_count(&buf).expect("counts");
        let e = assert_arp_pingpong(&frames, total, &one_round_capture())
            .expect_err("a runt reply must fail");
        assert!(e.contains("runt"), "names the runt: {e}");
    }

    // ---- 08-06: the netip static gate's assertions (NETIP-04) ---------

    /// A gratuitous ARP request (the devip sendgarp shape): oper 1,
    /// sha = the guest MAC, spa == tpa == the guest address.
    fn gratuitous_arp() -> Vec<u8> {
        let mut f = arp_request(); // oper 1, sha/spa guest, tpa gateway
        f[38..42].copy_from_slice(&GUEST_IP); // tpa == spa (who-has-self)
        f
    }

    /// A minimal ICMPv4 echo frame (eth IPv4, proto 1, 8-byte ICMP).
    fn icmp_frame(src_mac: [u8; 6], dst_mac: [u8; 6], src: [u8; 4], dst: [u8; 4], ty: u8) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&dst_mac);
        f.extend_from_slice(&src_mac);
        f.extend_from_slice(&[0x08, 0x00]); // ethertype IPv4
        f.extend_from_slice(&[
            0x45, 0x00, // v4 + IHL 20, tos
            0x00, 28,   // total len
            0x00, 0x00, 0x00, 0x00, // id/frag
            64, 1, // ttl, proto ICMP
            0x00, 0x00, // cksum (unchecked by the classifier)
        ]);
        f.extend_from_slice(&src);
        f.extend_from_slice(&dst);
        f.extend_from_slice(&[ty, 0x00]); // ICMP type, code
        f.extend_from_slice(&[0x00; 6]); // cksum + rest
        f
    }

    /// Golden: the gratuitous ARP appears (and only the guest's own
    /// request shape matches — the ping solicitation is not it).
    #[test]
    fn gratuitous_arp_golden() {
        let mut buf = global_header();
        buf.extend_from_slice(&record(&arp_request())); // the ping solicitation (tpa=gw)
        buf.extend_from_slice(&record(&gratuitous_arp())); // the sendgarp
        let frames = parse_pcap(&buf).expect("parses");
        assert_gratuitous_arp(&frames, GUEST_IP, BIND_LINE.as_bytes()).expect("the garp is found");
    }

    /// Negative: a capture with only the ping solicitation (tpa=gw) has
    /// no gratuitous ARP — fails naming the missing shape.
    #[test]
    fn gratuitous_arp_absent_fails() {
        let mut buf = global_header();
        buf.extend_from_slice(&record(&arp_request()));
        let frames = parse_pcap(&buf).expect("parses");
        let e = assert_gratuitous_arp(&frames, GUEST_IP, BIND_LINE.as_bytes())
            .expect_err("no who-has-self frame must fail");
        assert!(e.contains("gratuitous"), "names the missing garp: {e}");
    }

    /// Golden: one echo request + one reply passes with the request count.
    #[test]
    fn icmp_exchange_golden() {
        let req = icmp_frame(GUEST_MAC, GATEWAY_MAC, GUEST_IP, GATEWAY_IP, 8);
        let rep = icmp_frame(GATEWAY_MAC, GUEST_MAC, GATEWAY_IP, GUEST_IP, 0);
        let mut buf = global_header();
        buf.extend_from_slice(&record(&req));
        buf.extend_from_slice(&record(&rep));
        let frames = parse_pcap_icmpv4(&buf).expect("parses");
        assert_eq!(frames.len(), 2, "both frames classify");
        let n = assert_icmp_exchange(&frames, GUEST_IP, GATEWAY_IP, BIND_LINE.as_bytes())
            .expect("the exchange passes");
        assert_eq!(n, 1);
    }

    /// Negative: requests without a reply fail naming the missing reply.
    #[test]
    fn icmp_exchange_no_reply_fails() {
        let req = icmp_frame(GUEST_MAC, GATEWAY_MAC, GUEST_IP, GATEWAY_IP, 8);
        let mut buf = global_header();
        buf.extend_from_slice(&record(&req));
        let frames = parse_pcap_icmpv4(&buf).expect("parses");
        let e = assert_icmp_exchange(&frames, GUEST_IP, GATEWAY_IP, BIND_LINE.as_bytes())
            .expect_err("a reply-less exchange must fail");
        assert!(e.contains("reply"), "names the missing reply: {e}");
    }

    /// Negative: an echo request whose ethernet source is NOT the guest
    /// MAC fails (the guest never sent it).
    #[test]
    fn icmp_exchange_wrong_src_mac_fails() {
        let req = icmp_frame(GATEWAY_MAC, GATEWAY_MAC, GUEST_IP, GATEWAY_IP, 8); // wrong src_mac
        let rep = icmp_frame(GATEWAY_MAC, GUEST_MAC, GATEWAY_IP, GUEST_IP, 0);
        let mut buf = global_header();
        buf.extend_from_slice(&record(&req));
        buf.extend_from_slice(&record(&rep));
        let frames = parse_pcap_icmpv4(&buf).expect("parses");
        let e = assert_icmp_exchange(&frames, GUEST_IP, GATEWAY_IP, BIND_LINE.as_bytes())
            .expect_err("a wrong src_mac must fail");
        assert!(e.contains("src_mac"), "names src_mac: {e}");
    }
}
