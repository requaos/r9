---
phase: 07-r9-pci-ecam-virtio-net-devether
verified: 2026-11-05T16:07:53Z
status: passed
score: 38/39 must-haves verified
covered_files:
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-01-PLAN.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-01-SUMMARY.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-02-PLAN.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-02-SUMMARY.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-03-PLAN.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-03-SUMMARY.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-04-PLAN.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-04-SUMMARY.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-05-PLAN.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-05-SUMMARY.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-06-PLAN.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-06-SUMMARY.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-07-PLAN.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-07-SUMMARY.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-08-PLAN.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-08-SUMMARY.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-09-PLAN.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-09-SUMMARY.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-10-PLAN.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-10-SUMMARY.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-CONTEXT.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-EVIDENCE.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-REVIEW-DISPOSITION.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-REVIEW.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-UAT.md
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/evidence/07-07-virtnet-arp.log
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/evidence/07-07-virtnet-arp.pcap
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/evidence/07-07-virtnet-blast.log
  - .planning/phases/07-r9-pci-ecam-virtio-net-devether/evidence/07-07-virtnet-blast.pcap
  - build/aarch64/mkimage.sh
  - build/aarch64/netblast.c
  - build/aarch64/root-tree/lib/net-sweep.rc
  - build/aarch64/root-tree/lib/test/arp.req
  - build/aarch64/run-r9.sh
  - build/aarch64/smoke-r9.expect
  - build/aarch64/subset-build.sh
  - flake.lock
  - nix/boot-r9.sh
covered_digest: "v2:sha256:fe4db98fa89762e993b9e61922860a8f35d900a4937b94bc999d8bdd15c7e3a5"
behavior_unverified: 1
overrides_applied: 0
decision_coverage:
  honored: 10
  total: 10
  not_honored: []
re_verification:
  previous_status: human_needed
  previous_score: 33/34
  gaps_closed:
    - "G-07-1 round-2 root cause (RX frame counted but never enqueued): FIXED in code — pci.rs's RX drain slice base was KZERO+pa+VIRTIO_NET_HDR_LEN under a TWO-descriptor RX chain (hdr desc → shared header page; payload desc at pa), so the slice started 12 bytes INTO the frame (demux parsed etype 0x0a00≠2054, double skip, silent drop) and overread up to 108 bytes past the 4 KiB payload page on max-size frames. Fix: the extracted seam virtio_net_frame_offset() returns 0 (r9 3a49da1 RED seam+pins → a5b1b42 GREEN one-liner), pinned byte-exact against the evidence-pcap slirp reply through the REAL Ring + REAL drain arithmetic (port suite 54/54 lib, green live this session); both virtnet gates re-proven at a5b1b42 (arp in:4/out:4, blast N=256 NET-DONE — 07-EVIDENCE.md re-proof rows); composed-boot venue carries the fix (push-before-pin: r9 a5b1b42 == origin/main; flake.lock pin commit 91f6ab9aa names flake.lock ALONE; hosted r9-shell CI run 42058021884 SUCCESS at pushed HEAD 1dca03c25, REST-asserted live this session). The live guest re-confirmation (UAT test 1) remains pending with the user."
  gaps_remaining:
    - "Guest re-run of 07-UAT.md tests 1-2 (verify-work owns the reconciliation; both gaps stay open in 07-UAT.md until then)"
  regressions: []
behavior_unverified_items:
  - truth: "The data-file read path blocks per netif's qread semantics (Qmsg — one whole frame per read, virtio header stripped) via the EtherHooks { current, block, wake }"
    test: "Re-run 07-UAT.md test 1 against the composed boot at the new pin (`nix run .#boot-r9` — flake.lock now resolves the RX-fixed kernel a5b1b42): `bind -a '#l' /net`, clone, then `cat /net/ether0/$conv/data &` FIRST (holds the conversation open, inuse=1), THEN `echo connect 2054 > /net/ether0/$conv/ctl`. With no traffic the read must BLOCK; then dd /lib/test/arp.req from a second conversation (its own clone+connect 2054) so slirp replies, and confirm the blocked read wakes returning exactly one whole 64-byte frame with stats in: incremented."
    expected: "Block-on-empty, wake-on-first-matching-frame, one whole frame per read, stats in: live-incremented. The round-2 root cause (pci.rs:893's +12 slice base on the two-descriptor chain) is fixed at code level and host-pinned byte-exact at the exact seam whose absence let every prior gate assert counters while all inbound frames dropped; the corrected protocol (07-09) holds the conversation open across the connect."
    why_human: "The RX-side register→block→wake→whole-frame-copyout transition still has no behavioral test at any level: the rx_chain_tests pins cover the Ring↔pci.rs slice seam byte-exact (device write → drain → slice == the pcap reply), the 07-09 devether pin covers rx_ingress→pop_frame synthetically plus close→EOF-wake, but nothing exercises ingress→wake of a reader blocked on a data file (the publish-once constraint forbids a second full-path drive in the host binary — the 07-10 design note). Presence + the composed pin legs make a wedge implausible; they do not prove the transition. The guest re-run is the confirmation."
human_verification:
  - test: "Re-run 07-UAT.md test 1 with the corrected read-first-then-connect protocol (see behavior_unverified_items for the full protocol) against the composed boot at the new flake.lock pin (r9 a5b1b42 — the RX-fixed kernel)"
    expected: "Block-on-empty → wake-on-matching-frame → one whole 64-byte frame per read; stats in: incremented per frame; no wedge, no panic"
    why_human: "RX-side block→wake state transition with zero automated coverage at the transition itself; the round-2 root cause is fixed in code and host-pinned byte-exact at the previously-uncovered Ring↔pci.rs seam, but the live transition confirmation is the remaining step. G-07-1 stays open in 07-UAT.md until this re-run lands (verify-work owns the reconciliation)."
  - test: "Re-run 07-UAT.md test 2 against the composed boot at the new pin: `bind -a '#l' /net`, `echo connect -1 > /net/ether0/0/ctl`, then `cat /net/ether0/0/type`"
    expected: "Prints 4294967295 (the donor's byte-exact ` 4294967295 ` 12-byte field) — never the prior kernel panic. Fix verified in code (07-09): readnum_content truncates through u32 (devether.rs:496) exactly per the donor's int→ulong conversion, host-pinned byte-exact by devether_tests (green live this session within the 54/54 lib suite)"
    why_human: "The fix is host-test-proven and the full NTYPEQID path is wired (type_of → i64::from → readnum_content), but the guest-visible end-to-end readback at the pinned kernel is unexercised — the sweeps never connect -1. G-07-2 stays open in 07-UAT.md until this re-run lands."
deferred:
  - truth: "TX loopback/broadcast feedback into the input path (etheroq → etheriq)"
    addressed_in: "Phase 8"
    evidence: "07-CONTEXT phase boundary excludes arp/ethermedium/ipifc binding from Phase 7; ROADMAP Phase 8 SC4: 'arp + ethermedium bound to devether' (NETIP-04). The omission is documented loudly in devether.rs's header per the 07-07 must-have that locks it."
---

# Phase 07: r9 PCI/ECAM + virtio-net + devether — Verification Report

**Phase Goal:** r9 speaks Ethernet on qemu virt — PCI/ECAM discovery driven entirely by the committed DTB, a virtio-net driver with Day-1 zero-offload discipline over barrier-first virtqueues, and devether presenting `/net/ether0`, proven by a guest ARP request captured on the host. Kernel work in sibling repo `../r9`.
**Verified:** 2026-11-05T16:07:53Z
**Status:** human_needed
**Re-verification:** Yes — third verification, after gap-closure plan 07-10 (G-07-1 round 2: RX payload-offset fix)

**Verification posture note:** this re-verification re-proved the 07-10 delta live at r9 HEAD `a5b1b42ec92d5f6be4e13c1fe370da1768316131` (== origin/main == the flake.lock locked rev): the full port host suite ran green this session (117 tests across all port binaries; lib 54/54 = +3 vs the 51 at the 07-09 close) including all three new rx_chain pins and all four 07-09 devether pins; the xtask regression leg ran green (exit 0 — the pci.rs edits compile in the kernel-bin legs); the push/pin/CI chain was re-asserted against live git state and the GitHub REST API (run 42058021884 completed/success at headSha `1dca03c251d3…`). The standing phase evidence (truths 1–11, 14–27) was regression-checked against the 07-10 diff scope (`1c16193..a5b1b42`: pci.rs +14/-4 confined to the RX-drain region :878-903, virtio.rs +15 seam-only, lib.rs +5 registration, rx_chain_tests.rs +250 new — nothing else), which cannot touch the discovery/negotiation/ring-order/gate surfaces those truths stand on; the virtnet gates were re-proven by 07-10 at this exact HEAD (07-EVIDENCE.md re-proof section — HEAD == the gated SHA with zero intermediate commits) and were not re-run here per the dispatch. The harvey-side gate inputs are untouched since those runs (the interleaved Phase-6 commit `0f10a351c` touches only `build/aarch64/gethunk-repro-6l.sh` + `sys/src/BUILD_SETUP_NAMESPACE`; the later local commits are docs-only).

## Goal Achievement

### Observable Truths (phase contract — regression-checked)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | r9 finds the NIC from data, not constants: ECAM window from the DTB pci node's translated `reg`, device found by bus walk (1af4:1041 rev≥1), route decoded from `interrupt-map` — zero hardcoded IRQs/addresses (NETDRV-01) | ✓ VERIFIED | Unchanged by 07-10 (diff is RX-drain/seam-only). Prior live evidence stands: pci.rs:77-177 DTB parse; live log `pci: ecam base 0x4010000000`, `pci: virtio-net 1af4:1041 at 00:01.0 rev 01`, `pci: inta -> spi 4 intid 36` |
| 2 | Moved-window DTB perturbation still binds the device (NETDRV-01's proof) | ✓ VERIFIED | Committed `virt-gicv2-lowecam.{dts,dtb}` + `config_virtlowecam.toml`; prior live re-run PASS (`ecam base 0x3f000000`, same route, bind line present). 07-10 diff does not touch discovery |
| 3 | DTB discovery failure is loud, never a fallback (NETDRV-01 prohibition, judgment-tier — non-authoritative verdict) | ✓ VERIFIED (code evidence) | pci.rs header "deliberately NO hardcoded-window fallback"; named absence lines; host test `loud_absence_returns_none_never_a_default` in the port suite — green this session |
| 4 | The virtio transport is from-scratch in `port/src/virtio.rs` (D-01), generic (D-02), `#![forbid(unsafe_code)]` | ✓ VERIFIED | forbid attributes re-asserted live this session (virtio.rs:54, lib.rs 24 occurrences incl. the module-level re-assertions); the 07-10 seam addition is a `pub const fn` — no unsafe |
| 5 | Negotiation accepts exactly MAC(5)\|STATUS(16)\|VERSION_1(32) — zero-offload from Day 1 (D-03; NETDRV-02 prohibition, judgment-tier — non-authoritative verdict, test-backed) | ✓ VERIFIED | ACCEPT_LO/HI masks unchanged (the 07-10 virtio.rs diff is the +15-line seam only); `negotiate_accepts_exactly_mac_status_and_version1` green this session; gate log shows `negotiated 0x100010020` |
| 6 | Barrier-first ring code, order proven by test (D-04 first half) | ✓ VERIFIED | The order-pinning virtio_tests green this session within the 54/54 lib suite (visible in the live xtask output: `tx_post_barrier_sandwich_is_the_2_7_13_order`, `used_drain_is_acquire_ordered_and_recovers_the_slot`, etc.) |
| 7 | D-01 provenance decision recorded in-repo (NETDRV-02's record requirement) | ✓ VERIFIED | HowItWorks.md:242-248 unchanged |
| 8 | virtio_net_hdr is 12 bytes under VERSION_1 with a const size assertion | ✓ VERIFIED | const assert unchanged; the 07-10 seam doc comment restates the 12-byte/VERSION_1 truth (virtio.rs:835-848) |
| 9 | devether `#l` presents the netif.h file tree per NETQID(i,t)=(i<<5)\|t | ✓ VERIFIED | devether.rs metadata table/namec arm untouched by 07-10 |
| 10 | Clone/open semantics per netif.c:158-197 | ✓ VERIFIED | Exercised by the gates; unchanged |
| 11 | Write path netif-faithful: 60..=1514 bounds, src-MAC overwrite, block-on-full with wake (D-08) | ✓ VERIFIED | Blast gate re-proven at a5b1b42 by 07-10 (EVIDENCE re-proof: N=256 NET-DONE, same HEAD, no intermediate commits); the 07-10 diff is RX-side only |
| 12 | Data-file read path blocks per Qmsg semantics via EtherHooks | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | **Updated (round 2 FIXED at code level):** the round-2 root cause — pci.rs:893's `+ VIRTIO_NET_HDR_LEN` slice base on the two-descriptor RX chain, shifting the parse 12 bytes into the frame (etype 0x0a00≠2054 → double demux skip → counted at entry, never enqueued, silent; .planning/debug/rx-ingress-frame-never-enqueued.md) — is FIXED: `virtio_net_frame_offset()` → 0 (pci.rs:899 rides the seam), host-pinned byte-exact against the evidence-pcap reply through the REAL Ring + REAL drain arithmetic (rx_chain_tests green live this session), plus the max-size overread pin. The block→wake transition itself remains unexercised by any automated test — the guest re-run is pending. See Human Verification #1 |
| 13 | RX retire → demux per etheriq: inpackets++ every frame, type-match delivery, live counters | ✓ VERIFIED | arp/blast gate evidence stands and was re-proven at a5b1b42 by 07-10 with counters reconciled exactly as predicted (`in:4` — counts at rx_ingress entry, BEFORE the demux, so unchanged now that delivery works); delivery itself now host-pinned byte-exact at the slice seam + the 07-09 synthetic rx_ingress drive |
| 14 | ctl depth per discretion: connect N functional, promiscuous no-op, others loud-unimplemented, unknown verbs error loudly | ✓ VERIFIED | CR-01 fix (07-09) unchanged by 07-10: readnum_content truncates through u32 (devether.rs:496), host-pinned byte-exact, green live this session; live guest re-confirmation routes via Human Verification #2 |
| 15 | ifstats routes to the driver hook with the `vq<q>: avail= used= lastused= nintr= nnote=` line (NETDRV-04) | ✓ VERIFIED | Gate logs stand; re-proven at a5b1b42 by 07-10 (used==lastused per queue: vq0 4/4, vq1 4/4) |
| 16 | The guest's ARP request is observable in a host-side capture (NETDRV-03, D-06) | ✓ VERIFIED | Committed pcaps independently parsed (8 frames = 4 requests + 4 slirp replies, field-exact); arp gate re-proven PASS at a5b1b42 by 07-10; the PCAP_REPLY pin vector is byte-exact packet 2 of the committed pcap (independently verified by the 07-10 incremental review) |
| 17 | INTx level-trigger plumbing in BOTH GIC generations, ICFGR before ISENABLER | ✓ VERIFIED | gicv2.rs:95-108 / gicv3.rs:226-239 unchanged |
| 18 | ISR read-to-ack: handler's first device touch is the ISR byte | ✓ VERIFIED | pci.rs:847 `read_reg8(&state.isr, 0)` intact — the 07-10 pci.rs hunks start at :878, below the ack |
| 19 | Interrupt accounting reconciles under sustained load — no silent virtqueue wedge (NETDRV-04) | ✓ VERIFIED | Blast gate re-proven at a5b1b42 by 07-10 (N=256, NET-DONE, pcap assertion, soverflows 0 — 256×64 B ≈ 16 KiB against the 64 KiB store cap with delivery now working) |
| 20 | xtask netdev stanza is aarch64-virt-only; other-arch argv untouched (D-09/D-10) | ✓ VERIFIED | config.rs:576-584 unchanged |
| 21 | xtask pcap parser is std-only and unit-proven (D-06 machinery) | ✓ VERIFIED | pcap.rs tests unchanged; xtask suite green live this session (exit 0) |
| 22 | Bootfs payload: 60-byte ARP fixture, two-mode sweep, netblast, dd — manifest-asserted | ✓ VERIFIED | Harvey-side files unchanged (fingerprint-covered) |
| 23 | virtnet Net profile: real Harvey boot chain, typed sweep, fast-fail, pcap assertions in finish() | ✓ VERIFIED | Both modes re-proven green at a5b1b42 by 07-10 (EVIDENCE re-proof section) |
| 24 | Driver bind per spec order: BAR4 from the DTB `ranges` window, PCI_COMMAND explicit, caps bounds-checked, queue_enable LAST, DRIVER_OK before notify, RX pre-post | ✓ VERIFIED | Order pinned by host test (green this session); live bind lines stand |
| 25 | `dev = ["ether"]` across every aarch64 virt config + cfg(dev_ether) declared | ✓ VERIFIED | Unchanged |
| 26 | Push-before-pin: r9 pushed BEFORE the flake.lock advance; pin commit names flake.lock alone; locked rev == live HEAD, owner requaos | ✓ VERIFIED | **Re-verified live at the new pin:** r9 HEAD == origin/main == `a5b1b42ec92d5f6be4e13c1fe370da1768316131` (rev-parse + ls-remote this session); harvey commit `91f6ab9aa` touches flake.lock ALONE (diff-tree verified this session); flake.lock r9 block: rev `a5b1b42…`, owner `requaos` |
| 27 | Composed boot speaks Ethernet: identical stanza on all three compose paths, smoke nic-bind post-pass, fresh hosted CI green | ✓ VERIFIED | Stanza/post-pass unchanged. **Hosted CI REST-verified this session: run 42058021884 completed/success, headSha `1dca03c251d3…` (the pushed evidence-rows commit carrying the pin); tip run 42058871366 also success at `0f10a351c` (the interleaved Phase-6 commit, which carries these commits)** |

### Gap-Closure Plan 07-09 Must-Haves (standing — quick regression check)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 28 | readnum_content reproduces the donor's 32-bit truncation exactly: `connect -1` reads back as the 12-byte field ` 4294967295 ` | ✓ VERIFIED | devether.rs:496 `let v = val as u32 as u64;` unchanged by 07-10; the three byte-exact pins **green live this session** within the 54/54 lib suite |
| 29 | The doc comment states the true donor shape (32-bit ulong truncation, no sign extension) | ✓ VERIFIED | devether.rs:475-491 unchanged by 07-10 |
| 30 | conv_close's last-close reset pinned by a crate-root host test as INTENTIONAL donor behavior | ✓ VERIFIED | `conv_close_last_close_reset_is_donor_intentional` green live this session |
| 31 | 07-UAT.md test 1 carries the corrected protocol (read FIRST, then connect) | ✓ VERIFIED | 07-UAT.md:28-29 unchanged; the file's last touch predates 07-10 (git log confirms 07-10's three harvey commits never name it) |
| 32 | 07-REVIEW-DISPOSITION.md CR-01 marked fixed; WR-05/WR-06 open and untouched | ✓ VERIFIED | Byte-untouched by 07-10 (diff-tree isolation of all three 07-10 harvey commits confirmed live); CR-01 `fixed`, open 13/14 |
| 33 | Full port host suite green, xtask regression green, both virtnet gates green on the fixed HEAD | ✓ VERIFIED | **Re-run live this session at a5b1b42:** 117 passed / 0 failed across all port binaries (lib 54/54); `cargo xtask test` exit 0. Gates re-proven at a5b1b42 by 07-10 itself (same HEAD, no intermediate commits) |
| 34 | Push-before-pin carries the fix to the composed-boot venue | ✓ VERIFIED | Superseded by the 07-10 advance (truth 26/38): the 1c16193 pin chain was verified live in the prior round; the a5b1b42 chain is verified live this round |

### Gap-Closure Plan 07-10 Must-Haves (the delta — full verification)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 35 | The RX drain hands rx_ingress the frame the device actually wrote: slice base = KZERO + pa + `virtio::virtio_net_frame_offset()` with the offset seam named in port::virtio (G-07-1 round-2 binding diagnosis) | ✓ VERIFIED | **Live this session:** virtio.rs:846-848 `pub const fn virtio_net_frame_offset() -> usize { 0 }` with the two-descriptor chain-truth doc comment (§5.1.6.3.2, the hard I/O-vector boundary, qemu's iov_from_buf); pci.rs:899 slice base rides the seam; the demux (:881-889) and SAFETY (:891-896) comments tell the header-was-never-in-this-buffer truth; RED→GREEN order in the r9 log (`3a49da1` test → `a5b1b42` fix); RED evidence record present (`.planning/tmp/07-10-red-evidence.json`, exit 101, target the byte-exact pin) |
| 36 | A chain-faithful host pin covers the previously-uncovered Ring↔pci.rs seam: device-write simulation per §5.1.6.3.2 (the EXACT 64-byte evidence-pcap slirp reply into the payload page at offset 0, used entry {id 0, len 76}) → real Ring drain → real slice arithmetic → byte-exact; a max-size arm (used len 4108) proves no overread past the 4 KiB page | ✓ VERIFIED | rx_chain_tests.rs (250 lines, crate-root, cfg(test)-gated at lib.rs:87-88): `rx_chain_layout_is_hdr_descriptor_then_payload_descriptor` (the chain-shape premise), `rx_drain_slice_delivers_the_device_written_frame_byte_exact` (PCAP_REPLY byte-exact, :66-71 = the pcap packet-2 bytes), `rx_drain_slice_of_a_max_size_frame_stays_within_the_payload_page` — **all three green live this session**; the incremental review independently re-verified PCAP_REPLY against the committed pcap byte-for-byte |
| 37 | Full r9 port suite green, `cargo xtask test` regression leg green, and BOTH virtnet gates re-proven at the fixed HEAD (arp counters reconcile: in: still 4 — counts at rx_ingress entry) | ✓ VERIFIED | **Port suite re-run live this session: 117 passed / 0 failed across all port binaries (lib 54/54 = +3 over the 51 baseline at the 07-09 close, exactly the delta the plan predicts); `cargo xtask test` exit 0 live this session** (the pci.rs edits compile in the kernel-bin legs). Both virtnet gates re-proven at a5b1b42 by 07-10 (07-EVIDENCE.md re-proof: arp in:4/out:4 + pcap 8 frames + ifstats used==lastused + zero unhandled-irq; blast N=256 NET-DONE + soverflows 0) — HEAD == the gated SHA, zero intermediate commits, not re-run here per dispatch |
| 38 | The composed-boot venue carries the fix (push-before-pin): r9 RED+GREEN pushed to requaos/r9 main FIRST, flake.lock advanced SECOND as its OWN isolated commit (locked rev == live HEAD == origin/main, owner requaos), fresh hosted r9-shell CI green at the pushed Harvey HEAD | ✓ VERIFIED | **All legs live this session:** r9 ls-remote origin main == HEAD == `a5b1b42…` with plan-scope clean (only the concurrent session's exec.rs dirty); `91f6ab9aa` diff-tree names flake.lock ALONE; flake.lock r9 block rev `a5b1b42…` owner `requaos`; **CI run 42058021884 completed/success at headSha `1dca03c251d372661eceafc90b2cec858c24e107` (GitHub REST API, live)** — and tip run 42058871366 success at `0f10a351c` carrying these commits |
| 39 | 07-EVIDENCE.md gains the 07-10 re-proof section + new Pinned-commits rows; 07-UAT.md NOT result-edited; 07-REVIEW-DISPOSITION.md byte-untouched | ✓ VERIFIED | EVIDENCE.md:84-89 the `### 07-10 — … re-proof at r9 a5b1b42e…` section (full SHA, both gate verdicts, the unchanged-captures line) + pin-table rows 17-18 (a5b1b42 with the `1c16193..a5b1b42` push range; `91f6ab9aa` naming flake.lock ALONE); UAT.md last touched by `38d2887b9` (pre-07-10) and DISPOSITION.md by `7e2531eb7` (07-09) per git log; all three 07-10 harvey commits diff-tree-isolated to EVIDENCE.md/flake.lock |

**Score:** 38/39 truths verified (1 present, behavior-unverified)

### Decision Coverage

All trackable CONTEXT.md decisions honored (10/10 — D-01..D-10; non-blocking gate). 07-10 re-applied the carried-forward push-before-pin decision at the fix HEAD (T-07-22's gate shape: push-first, isolated pin commit, fresh hosted CI).

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | TX loopback/broadcast feedback into the input path (etheroq → etheriq) — deliberately not implemented this phase | Phase 8 | 07-CONTEXT phase boundary excludes the arp/ethermedium/ipifc binding; ROADMAP Phase 8 SC4 ("arp + ethermedium bound to devether", NETIP-04); omission documented loudly in devether.rs header per the 07-07 must-have that locks it |

### Required Artifacts

r9-side artifacts are outside this repo's fingerprint surface (the phase pins the exact rev — r9 `a5b1b42ec92d5f6be4e13c1fe370da1768316131` == origin/main == flake.lock locked rev — as the evidence anchor; per-phase SHAs in 07-EVIDENCE.md).

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `../r9/aarch64/src/platform/virt/pci.rs` | DTB-driven ECAM discovery + driver bring-up + IRQ handler | ✓ VERIFIED | **07-10 delta verified:** RX drain slice base rides the seam (:899); demux + SAFETY comments tell the two-descriptor truth (:881-896); hunks confined to :878-903 — ISR ack (:847), discovery, bind order untouched |
| `../r9/port/src/virtio.rs` | from-scratch unsafe-free transport | ✓ VERIFIED | **07-10 delta verified:** `virtio_net_frame_offset()` at :846-848 beside `virtio_net_frame_len` (:831), body the bare `0`, chain-truth doc comment; `#![forbid(unsafe_code)]` intact; diff is +15 seam-only |
| `../r9/port/src/rx_chain_tests.rs` | **NEW (07-10):** crate-root chain-faithful pins | ✓ VERIFIED | 250 lines (≥ the 60-line must-have); the three exactly-named pins + PCAP_REPLY byte-exact vector; spec-fixed constants with §2.7 citations; the publish-once design note in the header; all green live this session |
| `../r9/port/src/lib.rs` | `#[cfg(test)] mod rx_chain_tests;` registration | ✓ VERIFIED | lib.rs:87-88 with the crate-root rationale comment beside the devether_tests precedent; cfg(test)-gated — vanishes from kernel builds (lib.rs:3 cfg_attr no_std) |
| `../r9/port/src/netif.rs` | Netif/Netfile model, counters, EtherHooks | ✓ VERIFIED | Untouched by 07-10 (diff stat confirms); conv_close semantics host-pinned (07-09) |
| `../r9/port/src/devether.rs` | the `#l` device surface | ✓ VERIFIED | Untouched by 07-10; the 07-09 readnum_content fix stands (pins green live) |
| `../r9/port/src/devether_tests.rs` | crate-root pins (07-09) | ✓ VERIFIED | All four green live this session within the 54/54 lib suite |
| `../r9/port/src/fdt.rs` | interrupt-map helpers + decode tests | ✓ VERIFIED | Unchanged; fdt tests green this session (visible in the live xtask output) |
| `../r9/port/src/virtio_tests.rs` | host ring/negotiation suite (D-04) | ✓ VERIFIED | 22 tests green this session |
| `../r9/xtask/src/pcap.rs` + `config.rs` + `main.rs` | netdev emission, Net profile, assertions | ✓ VERIFIED | Unchanged; gates re-proven by 07-10; xtask test leg green live |
| `../r9/HowItWorks.md` | D-01 provenance record | ✓ VERIFIED | §virtio lines 242-248 |
| `../r9/aarch64/lib/virt-gicv2-lowecam.{dts,dtb}` + `config_virtlowecam.toml` | the perturbation fixture | ✓ VERIFIED | Committed; unchanged |
| `build/aarch64/root-tree/lib/test/arp.req` | 60-byte ARP fixture | ✓ VERIFIED | Byte-exact vs D-07+errata (prior; file unchanged) |
| `build/aarch64/root-tree/lib/net-sweep.rc` | two-mode sweep | ✓ VERIFIED | Unchanged |
| `build/aarch64/netblast.c` | TX-bulk generator | ✓ VERIFIED | Unchanged |
| `evidence/07-07-virtnet-{arp,blast}.{log,pcap}` | committed gate evidence | ✓ VERIFIED | Unchanged (07-10 deliberately committed no capture refresh — re-proof rows only; PCAP_REPLY is byte-exact packet 2 of the arp pcap) |
| `flake.lock` | r9 pin at the fixed HEAD | ✓ VERIFIED | rev `a5b1b42…`, owner requaos — resolves the RX-fixed kernel from a clean clone |
| `07-UAT.md` | corrected test-1 protocol; gaps open pending re-run | ✓ VERIFIED | Byte-untouched by 07-10 per plan (verify-work owns the reconciliation); round-2 root_cause field carries the now-fixed diagnosis |
| `07-REVIEW-DISPOSITION.md` | CR-01 fixed, open 13/14 | ✓ VERIFIED | Byte-untouched by 07-10 per plan (this defect predates the review's findings list — never a numbered finding) |
| `07-EVIDENCE.md` | 07-10 re-proof section + new pin rows | ✓ VERIFIED | `### 07-10 — … re-proof at r9 a5b1b42e…` (full SHA, both gate verdicts, unchanged-captures line) + the two Pinned-commits rows |
| `nix/boot-r9.sh`, `build/aarch64/run-r9.sh`, `build/aarch64/smoke-r9.expect` | D-10 compose stanza + nic-bind post-pass | ✓ VERIFIED | Unchanged |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `pci.rs` RX drain | `virtio.rs` | slice base = `KZERO as usize + pa as usize + virtio::virtio_net_frame_offset()` — ONE named seam carries the chain-layout knowledge; the unsafe from_raw_parts stays arch-side | ✓ WIRED | pci.rs:899 + virtio.rs:846-848, live this session; port::virtio keeps #[forbid(unsafe_code)] |
| `rx_chain_tests.rs` | `virtio.rs` | the pin drives the REAL Ring (pci.rs:466-504's exact construction: qsize 8, RX, shared hdr_pa, rx_cap 4108) + the REAL drain arithmetic (virtio_net_frame_len / virtio_net_frame_offset) against a simulated device write | ✓ WIRED | Import at rx_chain_tests.rs:32; all three pins green live this session |
| `rx_chain_tests.rs` | `evidence/07-07-virtnet-arp.pcap` | PCAP_REPLY is the byte-exact 64 bytes of packet 2 (slirp's ARP reply, incl_len 64) | ✓ WIRED | Vector at rx_chain_tests.rs:66-71; independently re-verified byte-exact by the 07-10 incremental review (dst/src/etype/oper/spa/tha/tpa + 22 zero pad bytes) |
| `flake.lock` | `../r9` (requaos/r9 origin main) | push-before-pin: locked rev == live HEAD == origin/main, owner requaos | ✓ WIRED | Asserted live this session (ls-remote + flake.lock grep + diff-tree isolation of `91f6ab9aa`) |
| All prior links (devether_tests↔devether/netif, NTYPEQID→readnum_content, pci↔fdt, trap↔pci, main↔bind/hooks, namec↔devether, xtask↔pcap/config, sweep↔fixtures, smoke↔compose) | — | — | ✓ WIRED | Regression-checked: the 07-10 r9 diff (4 files, RX-drain/seam/registration/new-test only) cannot touch these surfaces; devether pins green live |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| RX frame slice (the 07-10 fix) | payload page bytes at offset 0..frame_len | device write into the two-descriptor chain (hdr → shared header page; frame → payload page) | Yes — byte-exact pinned against the real captured slirp reply through the REAL Ring; the overread cured by the same line | ✓ FLOWING |
| type file readback | stored conversation typ | ctl `connect` write → ConvBook → readnum_content | Yes — donor-exact over the full admitted domain [-1, 0xffff], host-pinned at the boundaries | ✓ FLOWING |
| pci discovery prints | ecam base/size, route | DTB parse | Yes (unchanged) | ✓ FLOWING |
| negotiated-mask print | features | device config registers | Yes (unchanged) | ✓ FLOWING |
| stats file | in/out counters | IRQ-driven RX retire + TX post | Yes (re-proven at a5b1b42 by 07-10's gates; `in:` counts at entry, pre-demux — unchanged with delivery working) | ✓ FLOWING |
| ifstats file | avail/used/lastused/nintr/nnote | live ring indices + IRQ counters | Yes (re-proven at a5b1b42: used==lastused per queue) | ✓ FLOWING |
| host pcap | frames | qemu filter-dump | Yes (unchanged) | ✓ FLOWING |

### Behavioral Spot-Checks (this session)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| **The three 07-10 rx_chain pins** | `nix develop --command cargo test -p port` (r9 @ a5b1b42) | `rx_chain_layout_is_hdr_descriptor_then_payload_descriptor … ok`, `rx_drain_slice_delivers_the_device_written_frame_byte_exact … ok`, `rx_drain_slice_of_a_max_size_frame_stays_within_the_payload_page … ok` | ✓ PASS (live) |
| Full port host suite, zero regressions | same run | 117 passed / 0 failed across all port binaries (lib 54/54 = +3 vs the 51 baseline at the 07-09 close — exactly the plan's predicted delta; all four 07-09 devether pins still green) | ✓ PASS (live) |
| xtask regression leg (kernel-bin compile of the pci.rs edits) | `nix develop --command cargo xtask test` | exit 0 | ✓ PASS (live) |
| r9 push state | `git rev-parse HEAD` + `git ls-remote origin main` + plan-scope status | both `a5b1b42ec92d5f6be4e13c1fe370da1768316131`; the four plan-scope files clean (only the concurrent session's exec.rs dirty); RED `3a49da1` directly beneath GREEN `a5b1b42` | ✓ PASS (live) |
| Pin isolation + lock accuracy | `git diff-tree --name-only -r 91f6ab9aa`; grep -A8 '"r9"' flake.lock | `flake.lock` alone; rev == HEAD, owner requaos | ✓ PASS (live) |
| 07-10 docs-commit isolation | `git diff-tree --name-only -r c0b3abe1c` / `1dca03c25` | both name 07-EVIDENCE.md ALONE (07-UAT.md / 07-REVIEW-DISPOSITION.md byte-untouched, confirmed by git log) | ✓ PASS (live) |
| Hosted CI at the pushed HEAD | GitHub REST `/actions/runs/42058021884` | `completed success 1dca03c251d372661eceafc90b2cec858c24e107 r9-shell` (+ tip run 42058871366 success at `0f10a351c` carrying these commits) | ✓ PASS (live) |
| RED evidence record | `.planning/tmp/07-10-red-evidence.json` | present: command = the real filtered run, exit 101, target = the byte-exact pin (RED_EVIDENCE_OK was recorded before GREEN per the SUMMARY) | ✓ PASS (live) |
| virtnet arp + blast gates at a5b1b42 | `cargo xtask boot-test --arch aarch64 --config virtnet` (± blast) | PASS ×2 at a5b1b42 — re-proven by 07-10, recorded in 07-EVIDENCE.md; not re-run this session per dispatch (HEAD == the gated SHA, zero intermediate commits, harvey-side gate inputs untouched) | ✓ PASS (committed re-proof) |

### Test Quality Audit (07-10 additions)

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
|-----------|-----------|--------|---------|----------|-----------------|---------|
| `port/src/rx_chain_tests.rs` (3 pins) | NETDRV-03 | 3 | 0 | No — oracles are the virtio spec (§5.1.6.3.2 chain semantics, §2.7 ring layout constants cited per-constant) and the committed evidence pcap (PCAP_REPLY byte-exact, independently re-verified by the incremental review); the drive uses the REAL Ring + REAL drain arithmetic, never a reimplementation; RED-proven (test 2 failed on the byte-exact assert with the shifted frame as the diff's left side; test 3 panicked `range end index 4108`; RED_EVIDENCE_OK recorded) | Value-level (byte-exact 64-byte slice equality + descriptor-field reads + bounds behavior) | ✓ SUFFICIENT |

**Disabled tests on requirements:** 0. **Circular patterns:** 0. **Insufficient assertions:** 0.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | — | No TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER markers in any 07-10-modified file; no bounds guard/clamp/min() snuck into the fix (the bare-zero body at virtio.rs:847 confirmed live — the 07-09-prohibition shape honored: next_used's used-len validation bounds frame_len ≤ 4096, so the slice is in-page by construction); the test module is cfg(test)-gated and cannot reach kernel builds | — | — |

### Review Findings Routing (07-REVIEW.md — 13 open/14 total in 07-REVIEW-DISPOSITION.md)

**CR-01 remains FIXED** (07-09). The G-07-1 round-2 defect (the +12 slice offset) was **never a numbered finding** — it predates the review's findings list — so the disposition ledger correctly gains no row (07-REVIEW-DISPOSITION.md byte-untouched by 07-10, per plan). The **incremental review pass on the 07-10 diff found 0 new findings** and independently verified: offset-0 correctness for the seam's sole production caller, the used.len−12 underflow impossibility (next_used's `len ≥ 12` validation), the pins' fidelity against the real Ring (incl. PCAP_REPLY byte-exactness against the committed pcap), and that no new panic path reaches kernel code.

The remaining 13 open findings (WR-01..WR-08, IN-01..IN-05) stand as previously routed: **WR-06 (RX retire slot-liveness) is now the most relevant follow-up** — the 07-10 fix makes the drain's free-stack push the live delivery path WR-06 threatens, and the 07-10 incremental review's notes reaffirm its remediation as the relevant follow-up; WR-05 (WaitSet 16 vs MAX_PROCS 64) likewise recommended before Phase 8's ethermedium lands; WR-01..04/WR-07/WR-08 cheap validation hardening; IN-01..05 hygiene. None falsifies a must-have truth; none was in 07-10's scope. They remain tracked in the disposition ledger — not verification gaps.

### Human Verification Required

**1. UAT test 1 re-run — blocking read path, corrected protocol, RX-fixed kernel (G-07-1)**

**Test:** Composed boot at the new pin (`nix run .#boot-r9` — flake.lock now resolves r9 `a5b1b42`, the RX-fixed kernel): `bind -a '#l' /net`, `conv=`{cat /net/ether0/clone}`, then **read first**: `cat /net/ether0/$conv/data &`, **then** `echo connect 2054 > /net/ether0/$conv/ctl`. With no traffic the read must block. From a second conversation (clone + connect 2054) `dd < /lib/test/arp.req > /net/ether0/$other/data` so slirp replies; confirm the blocked read wakes returning exactly one whole 64-byte frame and `cat /net/ether0/stats` shows in: incremented.
**Expected:** block-on-empty → wake-on-matching-frame → one whole frame per read; no wedge, no panic. (Round-2's failure mode — counted but never enqueued — is now impossible at the seam level: the slice is byte-exact pinned to what the device wrote.)
**Why human:** the RX-side register→block→wake transition has no automated test at the transition itself. The round-2 root cause is fixed in code and host-pinned byte-exact at the previously-uncovered Ring↔pci.rs seam (the reason every prior gate asserted counters while all inbound frames dropped); the corrected protocol holds the conversation open across the connect. The guest re-run is the remaining confirmation; G-07-1 stays open in 07-UAT.md until it lands (verify-work owns the reconciliation).

**2. UAT test 2 re-run — CR-01 fix live confirmation (G-07-2)**

**Test:** Same composed boot: `echo connect -1 > /net/ether0/0/ctl` then `cat /net/ether0/0/type`.
**Expected:** prints `4294967295` (the donor's byte-exact field) — never the prior `panicked at port/src/devether.rs:492:15` subtract-with-overflow.
**Why human:** the fix is verified in code and host-pinned byte-exact (green live this session), and the full NTYPEQID path is wired; but the guest-visible end-to-end readback at the pinned kernel is unexercised — no gate ever connects -1. G-07-2 stays open in 07-UAT.md until this re-run lands.

### Gaps Summary

**No verification gaps.** All automated evidence is green and the G-07-1 round-2 fix is verified in code: the RX drain's slice base is `virtio_net_frame_offset()` == 0 — the frame begins at the payload descriptor's offset 0 under the two-descriptor chain the driver itself builds — pinned byte-exact against the evidence pcap's real slirp reply through the REAL Ring and the REAL drain arithmetic (the seam whose absence let every prior gate assert counters while every inbound frame dropped). The same line cured the up-to-108-byte overread past the 4 KiB payload page (T-07-21). All three new pins plus the full port suite (117 tests, lib 54/54) and the xtask regression leg are green live this session; both virtnet gates were re-proven at the fixed HEAD by 07-10 (arp counters reconcile exactly as predicted: `in:4` counts at rx_ingress entry, pre-demux). The composed-boot venue carries the fix (r9 `a5b1b42` == origin/main; flake.lock pinned as its own isolated commit `91f6ab9aa`; hosted CI run 42058021884 success REST-verified live at the pushed HEAD `1dca03c25`).

The status is `human_needed` (not `passed`) solely because the two UAT gaps remain open pending the guest re-run of 07-UAT.md tests 1–2 with the user — test 1 with the corrected read-first-then-connect protocol against the now-RX-fixed kernel, test 2 expecting `4294967295` instead of the panic. Neither item contradicts the goal's achievement; they are live confirmations of code-verified fixes, and verify-work owns the reconciliation.

**Info (not a gap):** two harvey planning-docs commits (`515b2945c` plan completion, `edfa58a41` the 07-10 incremental review) are unpushed local master on top of the pushed `0f10a351c` — docs-only; the pin, evidence, and all substantive commits are pushed and CI-covered. The interleaved Phase-6 commit `0f10a351c` (pushed) touches only `build/aarch64/gethunk-repro-6l.sh` + `sys/src/BUILD_SETUP_NAMESPACE` — no phase-07 gate input. The pre-existing `cargo test --workspace` arch-crate host-link failure is untouched and structural; the sanctioned lanes (`-p port`, `-p xtask`) are green.

---

_Verified: 2026-11-05T16:07:53Z_
_Verifier: the agent (gsd-verifier)_
