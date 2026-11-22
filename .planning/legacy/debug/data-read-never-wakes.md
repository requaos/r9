---
status: investigating
trigger: "Gap G-07-1: read of /net/ether0/N/data blocks on empty, never wakes when inbound frame arrives (stats in: increments at device). Goal: find_root_cause_only."
created: 2026-11-05T00:00:00Z
updated: 2026-11-05T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED — the conversation's ether-type filter was reset to 0 by conv_close (netif.rs:497/499) when the `echo connect 2054 > ctl` redirection closed the last open reference; the demux gate at netif.rs:603 then skipped the conversation, delivered stayed false, and rx_ingress returned early at netif.rs:621-623 before the waiter drain/wake at 624-633.

reasoning_checkpoint:
  hypothesis: "conv_close's last-close reset clears slot.typ to 0 after the connect redirection closes, so the ARP reply (etype 2054) matches no conversation, delivered=false, and the early return skips the rx_waiters drain — the blocked reader is never woken"
  confirming_evidence:
    - "netif.rs:584 is the ONLY inpackets++ site (grep-verified) and rx_ingress is called only from pci.rs:898 (IRQ RX drain) — observed in:1 proves the ISR→drain→rx_ingress entry ran; the break is downstream of entry"
    - "netif.rs:490-508 conv_close at inuse==0 sets book.typ=0 AND slot.typ=0, open=false, drains queue; donor 9k netif.c:414-446 does the same (f->type=0, qclose) — donor-faithful"
    - "netif.rs:603 demux gate `slot.typ != etype && slot.typ >= 0` skips typ=0 convs for etype 2054; donor devether.c:174 (`f->type == type || f->type < 0`) identical"
    - "netif.rs:621-623 `if !delivered { return; }` sits BEFORE the rx_waiters.take/wake at 624-633 — no match means no wake"
    - "UAT sequence opens+closes ctl around echo connect (rc redirection), dropping inuse to 0 BEFORE cat data & opens the data chan — typ was 0 for the entire blocked window"
    - "Symptom details all match: in:1 (counted at entry), out:1 (TX post), zero error counters (soverflows only count matched-but-full pushes), no wake, no EOF (conv never closed afterward)"
  falsification_test: "If the conversation's typ were provably 2054 at frame-arrival time (e.g., connect issued while data was already held open), the demux would match and the wake would fire — this symptom could not occur. Also falsified if any other site incremented inpackets (grep shows none)."
  fix_rationale: "The kernel implements the donor's netif semantics exactly; the UAT procedure violates the 'connected conversation' precondition by dropping the last reference between connect and the blocked read. Fix the test procedure (hold a reference across connect), not the kernel."
  blind_spots: "Wake path was not exercised live (demux never matched), so this diagnosis does not prove wake works end-to-end; however it is statically sound (sched() marks Blocked at proc.rs:1158-1161; pipe_wake readies by pid at 1401-1409) and the identical hooks are live-proven by the TX blast gate."
  candidate_causes:
    - "code: demux skip due to reset typ (CONFIRMED)"
    - "data/environment: none — counters and frame contents verified consistent"
    - "config: none — hook registration confirmed at main.rs:502-506 under dev_ether"
    - "test-procedure: connect-before-hold violates netif open-lifetime scoping (CONFIRMED as the operative cause)"
  and_gate: "No — single sufficient cause. The reset alone produces every observed detail. (A latent AND-term would only matter if typ had stuck: none found — wake path is sound.)"
next_action: return ROOT CAUSE FOUND (goal: find_root_cause_only — no fix)

## Symptoms

expected: block-on-empty → wake-on-frame → one whole 64-byte frame returned; stats in: incremented
actual: cat blocked on data read forever; second conv sent ARP req (out:1); slirp reply counted at device (in:1); blocked cat never woke, no bytes, no exit
errors: none — silent wedge
reproduction: 07-UAT.md test 1 (boot, bind -a '#l' /net, clone, connect 2054, cat data &, dd arp.req from second conv)
started: discovered during Phase 07 UAT (first time conv-queue delivery + blocked-reader wake was exercised)

## Eliminated

- hypothesis: ISR drains ring but never calls wake / wake called before enqueue (ordering inversion)
  evidence: rx_ingress (netif.rs:577-634) pushes to the FrameStore at 609-619 BEFORE taking rx_waiters at 624-625 and waking at 631-633; ordering is correct. The wake is gated on `delivered`, which was false — the demux never matched.
  timestamp: 2026-11-05T00:20:00Z
- hypothesis: clone/conv id mismatch — reader and delivery on different queue objects
  evidence: read_data (devether.rs:540-556) indexes netid(c.qid.path) into the same INGRESS.convs[] that rx_ingress demuxes (netif.rs:599); single BusyCell state, ids 0/1 consistent.
  timestamp: 2026-11-05T00:20:00Z
- hypothesis: WaitSet registration silently dropped (WR-05 shape — [u64;16] vs 64-slot proc table)
  evidence: exactly ONE waiter (pid 9) in a 16-slot set; register() dedups and cannot fill. WR-05 is a real capacity-staleness warning but cannot fire here.
  timestamp: 2026-11-05T00:20:00Z
- hypothesis: RX IRQ never fires; stats in: incremented by a poll loop
  evidence: grep shows `ig.inpackets += 1` exists ONLY at netif.rs:584 inside rx_ingress, and rx_ingress is called ONLY from pci.rs:898 in virtio_net_irq's RX drain. in:1 observed ⇒ the ISR fired and drained the ring.
  timestamp: 2026-11-05T00:20:00Z
- hypothesis: wake hook machinery broken (block parks, wake never readies)
  evidence: pipe_block_current → sched() marks caller Blocked+blocked_in_kernel (proc.rs:1158-1161); pipe_wake readies Blocked procs by pid (proc.rs:1401-1409); hooks registered at main.rs:502-506; the identical hook set is live-proven by the TX blast gate (block-on-full → tx_retired → wake, pci.rs:872-874 → netif.rs:640-652).
  timestamp: 2026-11-05T00:25:00Z

## Evidence

- timestamp: 2026-11-05T00:05:00Z
  checked: port/src/netif.rs (whole file)
  found: rx_ingress (577-634) counts inpackets at 584 (the ONLY site), demuxes per conv with gates `slot.open` (600), `slot.typ != etype && slot.typ >= 0` (603), `tome||multi||prom` (606); pushes to FrameStore (611); `if !delivered { return; }` at 621-623 BEFORE rx_waiters.take (624-625) and hook wake (631-633). conv_close (475-516) at inuse==0 resets book.typ=0 (497) and slot.typ=0 (499), open=false, drains queue.
  implication: any frame arriving while all convs have typ=0 (or open=false) is counted (in++) but never delivered and never wakes anyone — silently, with no error counters (drops only counted on matched-but-full push, 613-620).
- timestamp: 2026-11-05T00:08:00Z
  checked: port/src/devether.rs (whole file) + namec.rs:398-416 close dispatch
  found: read_data (540-556) loops pop_frame → Empty → register_rx_waiter(pid) → block() — register-under-IRQ-mask → block shape is correct. write_ctl connect (674-683) → conv_connect. close() (306-313) → conv_close for data/ctl qids, COPEN-gated by namec::close_chan (405-411).
  implication: every open/close of a data or ctl chan moves inuse; a shell redirection (`echo connect 2054 > ctl`) is open→write→close, so the close fires conv_close.
- timestamp: 2026-11-05T00:12:00Z
  checked: aarch64/src/platform/virt/pci.rs virtio_net_irq (838-952)
  found: ISR byte read first (847, read-to-ack); TX drain → tx_retired (873); RX drain per used entry: strip 12-byte virtio header, rx_ingress(frame) at 898, page returned to free stack (899-902); replenish (921-941). rx_ingress is the ONLY caller-visible RX ingress; inpackets proof stands.
  implication: the device-side path to rx_ingress is intact and ran (in:1).
- timestamp: 2026-11-05T00:15:00Z
  checked: donor semantics — harvey sys/src/9k/port/netif.c:398-452 (netifclose), 491-543 (openfile); sys/src/9/pc/devether.c:134-209 (etheriq)
  found: donor netifclose resets f->type=0 + qclose(f->iq) when --f->inuse==0 (line 446); donor etheriq demux is `f->type == type || f->type < 0` + `tome || multi || f->prom` (174-175) — NO inuse/open check, and a type-0 conv never matches etype 2054.
  implication: r9's netif.rs is donor-faithful on both the reset and the demux. The donor kernel would behave identically for the UAT's command sequence.
- timestamp: 2026-11-05T00:18:00Z
  checked: aarch64/src/main.rs:502-506 (EtherHooks registration), proc.rs:1386-1409 (pipe_current_pid / pipe_block_current / pipe_wake), proc.rs:1158-1161 (sched marks Blocked)
  found: hooks registered before first exec under cfg(dev_ether); block = sched() which sets Blocked+blocked_in_kernel; wake readies Blocked procs by pid list. Same hooks live-proven by the TX blast gate.
  implication: wake machinery is not the break; had the demux matched, the reader would have been woken.
- timestamp: 2026-11-05T00:22:00Z
  checked: UAT transcript ordering (07-UAT.md:15-42 + gap G-07-1)
  found: `echo connect 2054 > ctl` ran BEFORE `cat data &`. Timeline per conv: clone open (inuse=1) → close (inuse=0, reset) → ctl open (inuse=1) → connect typ=2054 → close (inuse=0, RESET typ=0, open=false) → data open (inuse=1, open=true, typ=0). Sender conv identical, plus dd closed its data chan (open=false) before slirp's reply arrived.
  implication: at frame-arrival time conv0 had typ=0 (skipped at netif.rs:603) and conv1 had open=false (skipped at 600-601). delivered=false → early return → no wake. Every observed detail (in:1, out:1, zero error counters, no wake, no EOF) reproduced statically.
- timestamp: 2026-11-05T00:26:00Z
  checked: port/tests/ directory listing
  found: no netif/devether host test file exists — the block/wake/delivery path had zero test coverage at any level (consistent with the UAT's "Why human" note).
  implication: no test contradicts the reading; a host test pinning last-close reset semantics would be new coverage.

## Resolution

root_cause: The UAT procedure violates netif's open-lifetime scoping: `echo connect 2054 > /net/ether0/0/ctl` opens and closes the ctl chan, and that close drops inuse to 0, so conv_close performs the donor netifclose's last-close reset — typ cleared to 0 (port/src/netif.rs:497 book.typ=0 and :499 slot.typ=0). The blocked `cat data &` then reopens the conversation with typ=0. When slirp's ARP reply (ethertype 2054) arrives, the rx_ingress demux gate at netif.rs:603 (`slot.typ != etype && slot.typ >= 0`) skips the conversation (and the sender's conv is open=false), so `delivered` stays false and rx_ingress returns early at netif.rs:621-623 — before the rx_waiters drain/wake at 624-633. The frame is counted (in:1 at netif.rs:584) but never enqueued and no reader is woken. The kernel is donor-faithful: 9k netif.c:446 resets f->type=0 on last close; 9/pc devether.c:174 demuxes on `f->type == type || f->type < 0`. Break line: netif.rs:603 (demux skip), caused by the reset at netif.rs:497/499; no-wake amplification: netif.rs:621-623.
fix: (find_root_cause_only — direction for plan-phase --gaps) Fix the UAT test-1 procedure to hold a conversation reference across the connect (e.g., start `cat data &` first, THEN `echo connect 2054 > ctl` — the ctl close leaves inuse=1, no reset; or keep the clone ctl fd open, the canonical Plan 9 pattern), then re-run: demux matches → push → rx_waiters drained → pipe_wake readies the cat (wake path statically sound + TX-side live-proven). Optionally add a port host test pinning conv_close's last-close reset (typ=0, queue drain, EOF wake) as documented donor semantics, and annotate G-07-1's Truth with the open-lifetime scoping. Kernel code change NOT indicated.
verification: static proof — all symptom details reproduced by code trace (in:1 without delivery, zero error counters, no wake, no EOF); competing hypotheses eliminated with grep-level evidence; donor comparison confirms kernel fidelity. Live confirmation pending re-run with corrected procedure (fix phase's job).
files_changed: []
