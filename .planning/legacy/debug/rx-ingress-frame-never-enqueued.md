---
status: diagnosed
trigger: "Gap G-07-1 ROUND 2: with corrected protocol (typ stuck, proven by typeinuse), slirp's ARP reply is counted (in:1) but never enqueued to the connected conversation — queue empty, no error counters, silent. Break is inside rx_ingress's parse->demux->push path or in the slice pci.rs hands it. Goal: find_root_cause_only."
created: 2026-11-05T12:00:00Z
updated: 2026-11-05T12:45:00Z
---

## Current Focus

hypothesis: CONFIRMED (host-test proven) — the RX drain in pci.rs builds the frame slice at `KZERO + pa + VIRTIO_NET_HDR_LEN` (pci.rs:893), but the driver posts RX as TWO-descriptor chains: a 12-byte header descriptor at a SHARED per-queue header page (hdr_pa) NEXT-chained to the payload descriptor at pa (virtio.rs:546-573, rx_replenish :696-697). The device writes the virtio_net_hdr into the header page and the frame at payload offset 0 (§5.1.6.3.2) — the +12 strip offsets into the frame itself. rx_ingress then parses real[12..] as the header: dst=real[12..18]=[08 06 00 01 08 00] (tome=false), frame[0]=0x08 even (multi=false), etype=real[24..26]=0x0a00=2560 != 2054 -> demux skips the conversation at BOTH netif.rs:603 and :606 -> delivered=false -> early return at :621-623 BEFORE the waiter drain. Counted at :584, never enqueued, zero error counters.

reasoning_checkpoint:
  hypothesis: "pci.rs:893's `+ VIRTIO_NET_HDR_LEN` slice-base offset assumes a single-buffer hdr+frame layout, but the two-descriptor RX chain (hdr desc at shared hdr_pa + payload desc at pa) puts the frame at payload offset 0, so the slice is the real frame shifted right 12 bytes and rx_ingress's parse/demux rejects it silently after counting"
  confirming_evidence:
    - "virtio.rs:557-563: EVERY RX slot's header desc gets ADDR=hdr_pa (one shared page, :502-505) LEN=12 FLAGS=NEXT|WRITE; payload desc ADDR written at post time to pa (:696) LEN=4096 — the device CANNOT put frame bytes at payload+12; the chain boundary is the I/O vector boundary"
    - "pci.rs:885-897: frame_len = used.len - 12 (correct) but slice base = KZERO + pa + 12 (wrong buffer offset — strips a header that is not in this buffer)"
    - "HOST TEST (scratch, since deleted): REAL Ring + REAL rx_ingress + EXACT 64-byte pcap reply — Arm A (pci.rs's +12 slice): pop_frame == Empty, stats 'in: 1', 'soft overflows: 0' (round-2 live symptoms reproduced exactly); Arm B (offset-0 slice): Pop::Frame(64), bytes byte-exact == pcap reply (demux/push path sound)"
    - "byte-level parse of the shifted slice against the pcap reply: tome=false ([08 06 00 01 08 00] != 52:54:00:12:34:56), multi=false (0x08 even), etype=0x0a00 != 2054 — double demux skip, delivered=false"
    - "the contiguous-layout alternative (device writes hdr+frame at payload+0) is REFUTED twice: (a) qemu's iov_from_buf fills desc-1 (exactly 12 bytes) with the hdr before touching desc-2; (b) under that layout pci.rs's slice would be CORRECT and the live round-2 symptom (counted, never enqueued, typ stuck) could not occur"
  falsification_test: "the host test's Arm A would have delivered the frame (Pop::Frame) if the offset were correct or the layout contiguous — it returned Empty; also falsified if the device-side chain semantics placed the hdr in the payload page (refuted above)"
  fix_rationale: "remove the `+ VIRTIO_NET_HDR_LEN` from the slice base at pci.rs:893 (the frame begins at the payload descriptor's offset 0; frame_len = used.len - 12 is already correct) — this addresses the root cause (where the frame IS) not a symptom"
  blind_spots: "the device-write layout is modeled per spec §5.1.6.3.2 + qemu's iov semantics, not observed on hardware; the live re-run after the fix is the end-to-end confirmation. Also noted (same line, secondary): the +12 makes the slice END at pa + used.len, so a max-size frame (used.len=4108) would read 108 bytes past the 4096-byte payload page — the same fix cures that overread."
  candidate_causes:
    - "code: pci.rs:893 slice-base offset (CONFIRMED — host test)"
    - "code: rx_ingress parse offsets (ELIMINATED — Arm B delivers the real frame byte-exact)"
    - "config: feature negotiation hdr size (ELIMINATED — VERSION_1 pins 12 bytes on both sides; TX pcap corroborates)"
    - "data: pcap reply bytes (consumed as the test vector — no anomaly)"
    - "environment: qemu/slirp frame padding (64-byte wire frame — consumed; no anomaly)"
  and_gate: "No — single sufficient cause. The +12 offset alone produces every observed detail (in:1, empty queue, zero error counters, silent, wake-path irrelevance)."
next_action: return ROOT CAUSE FOUND (goal: find_root_cause_only — no fix)

## Symptoms

expected: blocked read of /net/ether0/N/data wakes on the first matching inbound frame and returns one whole frame
actual: in:1 (rx_ingress entry counted), out:1, queue stays EMPTY (second foreground read also blocks), all error counters 0, reader never wakes — silent drop between ingress entry and conversation queue
errors: none — silent drop
reproduction: 07-UAT.md test 1 rounds 2-3 (bind, clone, cat data & FIRST, connect 2054 sticks — typeinuse proof —, dd arp.req from conv2, slirp reply arrives, pid 8 never wakes)
started: round 2 of G-07-1 UAT re-run after 07-09 (2026-11-05)

## Eliminated

- hypothesis: conv typ reset to 0 before frame arrival (round-1 root cause as COMPLETE explanation)
  evidence: round-2 protocol ran the blocked read FIRST, then connect 2054 succeeded AND STUCK — conv2's duplicate connect 2054 was rejected "devether: ether-type already in use" (typeinuse walks live conv state at that moment). typ==2054 at frame-arrival time is proven; still no delivery.
  timestamp: 2026-11-05T12:00:00Z
- hypothesis: wake machinery / waiter drain broken
  evidence: round-2 foreground probe read of the same conv's data ALSO blocked => the queue was EMPTY, so there was nothing to wake for; the break is BEFORE the waiter drain (netif.rs:621-633), which round 1 proved correct (push at 609-619 precedes take/wake at 624-633).
  timestamp: 2026-11-05T12:00:00Z
- hypothesis: virtio_net_hdr SIZE mismatch (10 vs 12 bytes, MRG_RXBUF)
  evidence: virtio.rs:764-767 + compile-time assert (:782) — VERSION_1 is required (D-03, negotiate accepts exactly MAC|STATUS|VERSION_1, virtio_tests.rs:367-376 proves MRG_RXBUF/CSUM/EVENT_IDX never accepted), and §5.1.6.1 makes the header 12 bytes under VERSION_1 regardless of MRG_RXBUF. TX path's clean on-wire arp.req (evidence pcap) corroborates 12-byte hdr agreement with qemu.
  timestamp: 2026-11-05T12:10:00Z
- hypothesis: runt/short-frame drop at netif.rs:585-587 (frame.len() < 14)
  evidence: used.len is bounds-validated to >= 12 by Ring::next_used (virtio.rs:747) and frame_len = used.len - 12; slirp's ARP reply is 60 bytes on the wire (evidence pcap) -> used.len=72, frame_len=60 >> 14. Also a runt drop would still leave the dst/etype parse as the operative gate only if len >= 14 — not applicable.
  timestamp: 2026-11-05T12:10:00Z
- hypothesis: multicast gate drop (netif.rs:593-595)
  evidence: slirp's reply is UNICAST to the guest MAC (pcap: dst 52:54:00:12:34:56, frame[0]=0x52 even) — the gate only fires for non-broadcast multicast. With CORRECT bytes the gate cannot fire. (With the +12-shifted slice, frame[0]=real[12]=0x08 — still even, so the gate :593 does NOT fire either; the drop happens later at the demux gates.)
  timestamp: 2026-11-05T12:15:00Z
- hypothesis: used.slot mis-indexing -> wrong PA -> garbage frame
  evidence: next_used slot recovery `((id & qmask) >> 1)` (virtio.rs:754) inverts setup_chains' head=2*slot exactly; rx_buf_pa[slot] bound at replenish (pci.rs:928). Even if it produced garbage, the mechanism below subsumes the symptom with the CORRECT pa.
  timestamp: 2026-11-05T12:15:00Z

## Evidence

- timestamp: 2026-11-05T12:05:00Z
  checked: port/src/virtio.rs Ring::new/setup_chains (:506-573), rx_replenish (:675-721), next_used (:730-757), VIRTIO_NET_HDR_LEN (:764-782)
  found: RX chains are TWO descriptors per slot: header desc ADDR=hdr_pa (ONE SHARED per-queue header page — "one header per queue suffices", :502-505, :557), LEN=12, FLAGS=NEXT|WRITE; payload desc ADDR=pa (the slot's 4KiB page, written at post time :696), LEN=RX_PAYLOAD_CAP=4096, FLAGS=WRITE. used_len_min=12, used_len_max=rx_cap=12+4096 (:518-521 via pci.rs:490).
  implication: per virtio §5.1.6.3.2/§2.7.5 the device writes the 12-byte hdr into the FIRST descriptor (the shared header page) and the frame into the SECOND descriptor AT OFFSET 0. used.len = 12 + frame_len. The frame NEVER occupies payload bytes 0..12 — it STARTS at payload+0.
- timestamp: 2026-11-05T12:05:00Z
  checked: aarch64/src/platform/virt/pci.rs virtio_net_irq RX drain (:875-912), bind RX pre-post (:650-678), bring_up_queue (:466-504)
  found: the drain computes frame_len = used.len - 12 (:885, correct) but builds the slice at `KZERO + pa + VIRTIO_NET_HDR_LEN` (:891-897) — i.e. payload offset 12 — with pa = rx_buf_pa[used.slot] = the payload page. The +12 assumes a SINGLE-buffer layout (hdr+frame contiguous in one buffer) that the two-descriptor chain does not produce.
  implication: the slice handed to rx_ingress is the real frame shifted right by 12 bytes (real[12..] ++ 12 stale/zero tail bytes). Parse: dst = real[12..18], etype = real[24..26].
- timestamp: 2026-11-05T12:10:00Z
  checked: evidence pcap 07-07-virtnet-arp.pcap via tcpdump -xx
  found: slirp's actual reply frame (60 bytes on wire incl. padding): dst 52:54:00:12:34:56 src 52:55:0a:00:02:02 etype 08 06, ARP reply htype 00 01 ptype 08 00 hlen 06 plen 04 oper 00 02 sha 52:55:0a:00:02:02 spa 0a 00 02 02 tha 52:54:00:12:34:56 tpa 0a 00 02 0f, then zero padding.
  implication: with the +12 shift: tome = ([08 06 00 01 08 00] == 52:54:00:12:34:56) = FALSE; multi = 0x08 & 1 = FALSE; etype = u16::from_be_bytes(real[24], real[25]) = 0x0a00 = 2560 != 2054. Demux skips the conv TWICE over (:603 etype mismatch AND :606 !(tome||multi||prom)); delivered=false; early return at :621-623 BEFORE the waiter drain. in++ already happened at :584. No push attempted -> soverflows stays 0. EXACT symptom match: in:1, queue empty, zero error counters, silent.
- timestamp: 2026-11-05T12:15:00Z
  checked: port/src/devether_tests.rs (the working synthetic drive)
  found: the 07-09 test drives rx_ingress DIRECTLY with a hand-built 14-byte frame (:97-107) — it never exercises the pci.rs slice construction. Ring host tests (virtio_tests.rs) likewise never simulate a device WRITING into the RX chain and then check what slice the arch side would build; the slice arithmetic lives in pci.rs (arch-side, cfg(dev_ether), MMIO) — outside all host coverage.
  implication: the wedge is exactly in the uncovered seam the spawn context suspected: "the REAL driver's buffer slicing". Synthetic drive passes because it bypasses the buggy offset.
- timestamp: 2026-11-05T12:35:00Z
  checked: SCRATCH host test (port/src/rx_repro_tests.rs, temporarily registered, run, then deleted — never committed): REAL Ring<BareMem> (qsize 8, RX, shared hdr_pa, rx_cap 4108 — pci.rs:466-504's exact construction) + REAL netif::rx_ingress + the EXACT 64-byte slirp reply extracted from the evidence pcap (packet 2, incl_len 64). Device simulation per §5.1.6.3.2: 12-byte hdr into the shared header page, frame into the payload page AT OFFSET 0, used entry {id: 0 (slot-0 chain head), len: 76}. Drain via next_used (slot 0 recovered, frame_len = 76-12 = 64).
  found: ARM A (slice built EXACTLY as pci.rs:891-897: payload base + 12, len 64): rx_ingress counted the frame (stats "in: 1") but pop_frame returned Pop::EMPTY and "soft overflows: 0" — the round-2 live symptoms reproduced byte-for-byte. ARM B (control: slice at payload offset 0, where the device actually wrote): Pop::Frame(64) with buf[..64] byte-exact == the pcap reply, stats "in: 2". `cargo test -p port --lib rx_repro` — 1 passed.
  implication: CONFIRMED. The divergence is pci.rs:893's `+ VIRTIO_NET_HDR_LEN` on the payload-page base. With the two-descriptor chain the driver itself builds, the frame is at payload offset 0; the +12 shifts the parse window 12 bytes into the frame. The netif demux/push/wake path is sound against the REAL frame (Arm B).
- timestamp: 2026-11-05T12:40:00Z
  checked: shifted-parse arithmetic against the exact pcap reply bytes
  found: slice[0..6] = real[12..18] = [08 06 00 01 08 00] (the real etype+ARP htype/htype bytes) -> tome=false, frame[0]=0x08 even -> multi=false, multicast gate :593 NOT fired (multi false); etype = u16::from_be_bytes(real[24], real[25]) = 0x0a00 = 2560 (bytes inside slirp's source-MAC retransmission in the ARP body) != 2054 -> skip at :603; ALSO !(tome||multi||prom) -> skip at :606. delivered=false -> return at :622 before rx_waiters.take.
  implication: the drop is deterministic for every unicast-to-us frame and every broadcast frame (shifted frame[0] is an etype byte, 0x08 even); literally ALL inbound frames are counted-then-dropped — consistent with "every gate green while every inbound frame is dropped".
- timestamp: 2026-11-05T12:42:00Z
  checked: alternative hypothesis "device writes hdr+frame contiguously at payload+0" (would make pci.rs correct)
  found: REFUTED twice — (a) qemu virtio_net_receive_rcu writes the hdr with iov_from_buf(in_sg, 0, hdr, 12): desc-1 is exactly 12 bytes, so the hdr fully occupies the shared header page and the frame write at chain offset 12 lands at desc-2 offset 0; the descriptor boundary is a hard I/O-vector boundary. (b) Under the contiguous layout pci.rs's +12 slice would be CORRECT and the live round-2 symptom (counted, queue empty, typ stuck) could not have occurred — but it did.
  implication: no alternative layout rescues pci.rs:893; the offset is wrong under the chain the driver builds.
- timestamp: 2026-11-05T12:43:00Z
  checked: secondary defect at the same line (noted for the fix phase)
  found: the +12 base makes the slice END at pa + used.len; with the ring's used_len_max = 12 + 4096 = 4108 (pci.rs:490), a max-size frame would read 108 bytes past the 4096-byte payload page (mapped, so no fault — stale neighbor bytes). The offset fix cures this overread too.
  implication: one-line fix, two defects cured.

## Resolution

root_cause: pci.rs:893 builds the RX frame slice at `KZERO + pa + VIRTIO_NET_HDR_LEN` — a single-buffer mental model — but the driver posts RX as TWO-descriptor chains (virtio.rs:546-573): a 12-byte virtio_net_hdr descriptor at the shared per-queue header page (hdr_pa), NEXT-chained to the payload descriptor at pa. Per virtio §5.1.6.3.2 the device writes the hdr into the FIRST descriptor (the header page) and the frame into the SECOND descriptor AT OFFSET 0; used.len = 12 + frame_len. The drain's frame_len = used.len - 12 (pci.rs:885) is correct, but the slice base pa + 12 starts 12 bytes INTO the frame: for slirp's 64-byte ARP reply the parse reads dst=[08 06 00 01 08 00] (tome=false, multi=false) and etype=0x0a00=2560 != 2054, so rx_ingress skips the connected conversation at netif.rs:603 AND :606, delivered stays false, and the function returns at netif.rs:621-623 BEFORE the waiter drain — the frame is counted at entry (netif.rs:584, the observed in:1), never enqueued (the observed empty queue / second blocked read), with no push attempted (the observed soverflows:0), silently. Host-test CONFIRMED with the real Ring + real rx_ingress + the exact pcap reply bytes: pci.rs's slice drops it (Empty, in:1, no error counters); the offset-0 slice delivers it (Frame(64), byte-exact).
fix: (find_root_cause_only — direction for plan-phase --gaps) In aarch64/src/platform/virt/pci.rs:891-897, drop the `+ virtio::VIRTIO_NET_HDR_LEN` from the slice base: the frame starts at the payload page's offset 0 (the header lives in the separate shared hdr_pa descriptor; only used.len includes it — the frame_len subtraction at :885 stays). One-line change (`KZERO as usize + pa as usize`); also cures the pa+used.len overread past the 4 KiB payload page for max-size frames. Follow-ups for the fix phase: (1) extend port host coverage with a chain-faithful RX drive (device writes hdr page + payload page; assert the arch-side slice arithmetic delivers) — the current seam (Ring tests never simulate a device write; devether tests bypass pci.rs) is why every gate stayed green while all inbound frames were dropped; (2) re-run 07-UAT test 1 (blocked read FIRST, then connect — the 07-09 protocol) for live end-to-end confirmation.
verification: host-test proof (Arm A reproduced in:1 + Empty + soverflows:0 from the exact pcap bytes through the real Ring/rx_ingress; Arm B proved demux sound, Frame(64) byte-exact); static byte-level parse matches every round-2 live observation; alternative contiguous-layout hypothesis refuted by qemu iov semantics AND by the live symptom's existence. Live qemu re-run pending the fix phase (not this session's goal).
files_changed: []
