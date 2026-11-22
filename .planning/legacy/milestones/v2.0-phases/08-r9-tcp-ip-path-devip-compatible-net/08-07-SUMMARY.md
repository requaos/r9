---
phase: 08-r9-tcp-ip-path-devip-compatible-net
plan: 07
subsystem: infra
tags: [smoltcp, devip, dhcp, bootp, udp-broadcast, icmpv6, ndp, ipv6, ipifc, ipconfig, ping, slirp, pcap, plan9, aarch64, r9, no_std]

# Dependency graph
requires:
  - phase: 08-r9-tcp-ip-path-devip-compatible-net (plans 08-01..08-06)
    provides: the contract doc (the OQ-1 PASS-path verdict in §12.6; the ICMPv6 chapter §8; the ipifc chapter §9), the D-05 event-driven seam + the waker bridge, the #I tree + conv model + icmp/udp/tcp protos + netlog/route, the ethermedium (EtherDev, the snoop-fed /net/arp view, the gratuitous ARP, the RX-IRQ wake), the D-17 /dev/random (ipconfig's truerand), the baked gate payloads + the netaddr.rc seam + the virtip profile
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: the live devether the ether leg binds (the virtio-net NIC under qemu -netdev user/slirp)
provides:
  - the NETIP-05 DHCP gate GREEN on slirp: the REAL unmodified ipconfig dhcpclient's DORA over /net/udp broadcast fidelity — the pcap carries the broadcast src-0.0.0.0:68 → 255.255.255.255:67 Discover/Request (guest MAC as L2 src) and the Offer/AK from the seam gateway, the lease (10.0.2.15) ties to the seam, the guest configures (selftab/route/ndb) and pings the gateway — labeled `netip: NOT-L2-TRUTH slirp venue`; the smoltcp dhcp socket NEVER wired
  - the NETIP-06 IPv6 gate GREEN on slirp: the devip ip6 surface (the icmpv6 full-packet bridge per P-E — parse-on-write/synthesize-on-read, the write-side ICMP_USEAD6 prefix; ipifc add6/ra6/try with the tentative DAD lifecycle) configures the guest via unmodified ipconfig; the NS/NA exchange + 3/3 ICMPv6 echo replies with the gateway are pcap-proven — same venue label
  - the OQ-1 bridge rule landed at the devip file surface (udp_write_data sources 0.0.0.0 in the absorbed-DHCP window — keyed on the self-table's accept-all flag) + host pins (the absorption + the broadcast TX shape)
  - the recorded slirp v6 probe verdict (netaddr.rc finalized: fec0::15/fec0::2; echo served, ND answered, no RAs observed) + the contract doc's dated §8/§9/§12.6 implementation notes (the bridge's recorded corners)
affects: [08-08 (aan + the tcpblast soak ride the ether leg the gates proved), 08-09 (evidence consolidation + push-before-pin; NETIP-05's shared-ID gate closes when 08-09 lands), Phase 9 (the tap leg re-runs the DHCP/static/IPv6 gates for the L2-truth verdict — D-10/D-15)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as estimateTokens.
actuals:
  tokens: 41000   # (130,269-char r9 diff + 12,572-char Harvey plan-file diff + ~17,000-char this SUMMARY + ~4,000-char STATE/ROADMAP/REQUIREMENTS edits) / 4 — vs estimate 58,000 (confidence: low)
  tasks: 2
  commits: 10     # 5 r9 (37e77fb, e57e5f7, 8a30e64, 8bb29a2, 89ab75f) + 4 Harvey production (a37b78509, 9f9a833da, 41bed1534 metadata, 089637446 the fixture fix) + 1 pending correction commit (this file's attribution fix).  The measured Harvey ledger window 15957628b..HEAD carries one concurrent phase-10 docs commit (3896817e3 — disjoint files; reconciled per the 08-06 precedent)
plan_head_before: 15957628bbd2332227ec3b6328e4db274b1c3c67   # Harvey ledger base (#3968)
plan_head_after: 9f9a833da4d5ee6355c1729e79d1e43903ab41b4    # Harvey HEAD at the first SUMMARY write — the metadata + fixture-fix + correction commits follow per the atomic close-out order

# Tech tracking
tech-stack:
  added: []
  patterns: [the absorbed-marker-keyed 0.0.0.0 source override (the OQ-1 PASS-path's file-surface form — accept-all IS the DHCP-in-progress marker), the full-packet icmpv6 bridge (parse-on-write/synthesize-on-read with the write-side-only ICMP_USEAD6 prefix), the tentative-lifc DAD lifecycle (recorded, never pushed until confirmed), the probe-then-pin gate discipline (the gate pins to the probed slirp reality, recorded in netaddr.rc + the SUMMARY), the seam-address-as-parameter pcap assertion extended to DHCP + ND (netaddr_slirp_v6)]

key-files:
  created: [build/aarch64/root-tree/lib/netip-dhcp.rc, build/aarch64/root-tree/lib/netip-v6.rc]
  modified: [../r9/port/src/devip/proto.rs (the OQ-1 udp broadcast rule + the icmpv6 module), ../r9/port/src/devip/ipifc.rs (add6/ra6/try + the v6 update_ip_addrs arms + find_local_ip's v6 arm), ../r9/port/src/devip.rs (the icmpv6 clone/dispatch/slots + the format_ip fix), ../r9/port/src/devip/phyadapt.rs (StackDev::mac), ../r9/port/src/devip_tests.rs (5 new pins), ../r9/xtask/src/pcap.rs (the BOOTP/DHCP + ICMPv6/ND classifiers + assert_dhcp_dora + assert_nd_exchange + 17 host pins), ../r9/xtask/src/main.rs (the dhcp + v6 sweep modes), ../r9/docs/devip-contract.md (the dated §8/§9/§12.6 notes), build/aarch64/mkimage.sh (the manifest asserts), build/aarch64/netaddr.rc (the finalized v6 seam), .planning/.../deferred-items.md (the 08-07 warning addendum)]

key-decisions:
  - "08-07: the OQ-1 bridge predicate keys on the self-table's accept-all flag (the addselfcache null-address side effect IS the absorbed-DHCP marker) — an unspecified-laddr conv sources 0.0.0.0 ONLY in that window; on an ADDRESSED iface it keeps smoltcp's source selection (the naive laddr-only predicate sourced 0.0.0.0 there — the wedge pin caught it)"
  - "08-07: the icmpv6 bridge's recorded corners (contract §8's dated note): the ICMP_USEAD6 prefix is WRITE-SIDE ONLY (the donor consumes it on write; reads deliver the full packet from the IP6 header — ipconfig's recvrahost reads a Routeradv from byte 0); the wire src is smoltcp's RFC 6724 selection (icmp sockets carry no per-send override — the pre-autoconf RS src=:: corner is unreachable with slaac OFF); NDISC traffic NEVER surfaces to the icmpv6 socket (accepts_v6 matches echo by ident only — ipconfig's recvra path sees nothing on r9, a recorded divergence; the ND exchange itself runs in smoltcp's machinery, pcap-visible, snoop-fed to /net/arp); the read-synth ttl is the conv's configured ttl (smoltcp exposes no received hop limit)"
  - "08-07: try = tentative recorded devip-side, NEVER pushed to smoltcp; the confirming add pushes (the DAD window's shape); NO DAD NS emission on try — the donor's own NS would land in the TX-snooped /net/arp as a WAIT entry for the tentative address and ipconfig's arp-scan DAD would read a FALSE DUPLICATE (ipv6.c:425-455)"
  - "08-07: the slirp v6 probe verdict (2026-11-06, the plan's sanctioned probe-first sequencing): slirp answers ND for fec0::2 AND serves ICMPv6 echo (3/3 replies); NO unsolicited RAs observed; the guest's one extra v6 frame is its own MLDv2 solicited-node join report — netaddr.rc finalized fec0::15/fec0::2, NETIP_V6_SLIRP_NA = true"
  - "08-07: the DHCP fixture runs plain `ip/ipconfig ether /net/ether0` (the binary's DEFAULT mode on an ether — maximum D-09 fidelity; dhcpwatch forks and sleeps the lease harmlessly through the gate window; renewal is the plan's recorded out-of-scope)"

patterns-established:
  - "The probe-then-pin gate: a specless venue behavior (slirp's v6 service surface) is probed on the live gate FIRST, the outcome recorded in the SUMMARY + the seam file, and the gate pinned to the probed reality (assert_nd_exchange's expect_na + the drift-is-loud arm)"
  - "The absorbed-marker-keyed bridge override: devip-side absorbed state (never pushed to smoltcp) keys wire behavior via the model's flags (accept-all), keeping smoltcp's own selection authoritative outside the window"
  - "The classifier-per-protocol pcap suite: BOOTP/DHCP + ICMPv6/ND classifiers beside ARP/ICMPv4, every assertion with golden + negative host pins, the seam values as parameters"

requirements-completed: [NETIP-06]  # NETIP-06's declarers (08-03, 08-07) both carry SUMMARYs — the #2388 shared-ID gate's ready subset.  NETIP-05 stays PENDING: 08-09 also declares it and has no SUMMARY yet (the gate blocks marking until the last declarer finishes)

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The NETIP-05 DHCP gate green on slirp: the unmodified ipconfig dhcpclient's DORA over /net/udp broadcast fidelity (broadcast src-0.0.0.0 Discover/Request with the guest MAC as L2 src, Offer/AK from the seam gateway, the lease tied to the seam), the guest configured (selftab/route/ndb) and pinging the gateway — labeled NOT-L2-TRUTH; the smoltcp dhcp socket never wired"
    requirement: NETIP-05
    verification:
      - kind: e2e
        ref: "R9_NETIP_SWEEP=dhcp cargo xtask boot-test --arch aarch64 --config virtip → PASS (the finish arm asserts the ether-bind/dhcp-ok/NOT-L2-TRUTH markers + NETIP-DONE + the pcap DORA assertion; lease 10.0.2.15)"
        status: pass
      - kind: unit
        ref: "cargo test -p port devip → 35/35 (the absorption pin + the file-surface broadcast TX pin); cargo test -p xtask pcap → 69/69 (9 assert_dhcp_dora pins)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The NETIP-06 IPv6 gate green on slirp: the devip ip6 surface (the icmpv6 full-packet bridge + ipifc add6/ra6/try) configures the guest via unmodified ipconfig; the NS/NA exchange + the guest's 3 echo requests with 3 replies are pcap-proven — labeled NOT-L2-TRUTH; SLAAC off"
    requirement: NETIP-06
    verification:
      - kind: e2e
        ref: "R9_NETIP_SWEEP=v6 cargo xtask boot-test --arch aarch64 --config virtip → PASS (1 NS + 1 NA + 3 guest echo requests in the pcap; the fixture's ipselftab/ping/arp-view assertions green)"
        status: pass
      - kind: unit
        ref: "cargo test -p port devip → 35/35 (the full-packet round-trip, the headers-prefix shape, the add6/ra6/try verb pin incl. the tentative lifecycle); cargo test -p xtask pcap → 69/69 (8 assert_nd_exchange pins + the v6 parse pin)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The 0.0.0.0 absorption + the OQ-1 broadcast TX pinned host-side (the absorption leaves zero smoltcp addresses + the netlog note; the Bootp-shaped headers-mode write leaves the adapter src 0.0.0.0 / dst 255.255.255.255 / 68→67; the Offer reads back prefixed)"
    requirement: NETIP-05
    verification:
      - kind: unit
        ref: "cargo test -p port devip → ipifc_dhcp_preconfig_absorbs_with_zero_iface_addresses_and_a_netlog_note + dhcp_broadcast_tx_shape_through_the_devip_file_surface pass"
        status: pass
    human_judgment: false
  - id: D4
    description: "The recorded slirp v6 probe verdict: netaddr.rc's guestv6/gwv6 finalized from the live probe (echo served 3/3, ND answered, no RAs observed, the guest's MLDv2 join the only extra frame) — the gate pins to the probed reality with the drift arm loud"
    requirement: NETIP-06
    verification:
      - kind: other
        ref: "the 2026-11-06 v6 gate run's pcap census (python3 record walk over target/boot-test-virtip.pcap: types 128×3/129×3/135/136 + the one hop-by-hop MLDv2 frame) + netaddr.rc's recorded comment"
        status: pass
    human_judgment: false

# Metrics
duration: 45 min
completed: 2026-11-06
status: complete
---

# Phase 8 Plan 07: The DHCP + IPv6 Address-Configuration Gates Summary

**Both address-configuration gates are GREEN on slirp with pcap proof and the not-L2-truth labels: NETIP-05's DHCP via the REAL unmodified ipconfig dhcpclient over /net/udp broadcast fidelity (the DORA exchange in the capture — broadcast src-0.0.0.0:68 Discover/Request, Offer/AK from the gateway, lease 10.0.2.15 tied to the seam, /net/ndb written; the smoltcp dhcp socket never wired), and NETIP-06's full-fidelity ether-leg IPv6 (the icmpv6 full-packet bridge — parse-on-write/synthesize-on-read per Pitfall P-E, the write-side ICMP_USEAD6 prefix, ipifc add6/ra6/try with the tentative DAD lifecycle — the guest configured by unmodified ipconfig, the NS/NA exchange plus 3/3 ICMPv6 echo replies with the gateway in the capture). The OQ-1 verdict landed at the file surface (the udp broadcast TX sources 0.0.0.0 in exactly the absorbed-DHCP window), netaddr.rc's v6 values are finalized from the recorded probe, and every prior virtip mode + all 8 regression profiles stay green.**

## Performance

- **Duration:** 45 min
- **Started:** 2026-11-06T01:40:30Z
- **Completed:** 2026-11-06T02:25:00Z
- **Tasks:** 2
- **Files modified:** 13 (9 r9 modified + 5 Harvey created/modified — the list under key-files — plus planning records)

## Accomplishments

- **Task 1 (NETIP-05):** the udp TX path's OQ-1 bridge rule landed at the devip file surface — a conv with an unspecified recorded laddr sends with `local_address: Some(0.0.0.0)` exactly when the self-table's accept-all flag (the addselfcache null-address side effect = the absorbed DHCP preconfig marker) is set; outside that window smoltcp's own source selection governs (the wedge pin caught the naive laddr-only shape sourcing 0.0.0.0 on an addressed iface).  The host pins: the absorption (zero smoltcp addresses + the D-07 netlog note + accept-all) and the file-surface DORA shape (announce `*!68` → `headers` → the Bootp write leaves src 0.0.0.0 / dst 255.255.255.255 / 68→67; the broadcast Offer reads back with the synthesized 52-byte Udphdr).  The gate: `ip/ipconfig ether /net/ether0` (the binary's default dhcpclient mode) configures the guest on slirp; the pcap assertion (`assert_dhcp_dora`) proves the ordered four-message exchange with the OQ-1 wire shapes and the lease tied to the seam's guestv4
- **Task 2 (NETIP-06):** the icmpv6 module (contract §8) — headers mode (headers=6) with the WRITE-SIDE 40-byte ICMP_USEAD6 prefix (the donor's icmpkick6 consumes it on write; reads deliver the full packet from the IP6 header, ipconfig's recvrahost shape), the full-packet bridge (split on write — the bare message to smoltcp's Ident-bound icmp socket with the donor's ident overwrite in non-headers mode; synthesize the IP6 header on read), the icmpstats6 file (12 tags + named per-type lines, holes absent per the donor's dead else-if), and the recorded corners in the module header + contract §8's dated note (smoltcp's RFC 6724 source selection — no per-send override; NDISC never surfaces to the socket — the recvra divergence recorded; no version validation on write — the donor overwrites).  ipifc's v6 verbs: add6 (pref2addr EUI-64 per ethermedium, the donor's error strings verbatim), ra6 (the 11 keywords recorded + the maxraint<minraint restore + the live status line), try's tentative lifecycle (recorded, NEVER pushed until the confirming add — and NO DAD NS emission, which would false-positive ipconfig's arp-scan DAD via the TX snoop).  The gate: static v6 config via unmodified ipconfig, /net/ipselftab lists fec0::15, ping -6 gets 3/3 replies from fec0::2, /net/arp carries the gateway's v6 entry post-ND; the pcap carries the guest's NS + the gateway's NA + the echo frames
- **The probe (the plan's sanctioned sequencing):** slirp answers ND for fec0::2 AND serves ICMPv6 echo there (3/3); NO unsolicited RAs in the window; the guest's only extra v6 frame is its own MLDv2 solicited-node group-join report at the address add.  netaddr.rc's fec0::15/fec0::2 are finalized from it (the in-file comment records the probe); `NETIP_V6_SLIRP_NA = true` pins the verdict with a loud drift arm
- **Regression surface:** all five virtip modes (loopback, tcp, static, dhcp, v6) green on the final tree; all 8 regression profiles (virt, virtlowecam, virtexec, virtexec3, virtmulti, virtnotes, virtpanic, virtnet) green; port suite 89/89 (devip pins 35/35), xtask suite 69/69

## Task Commits

Each task was committed atomically across the two repos:

**Task 1 (NETIP-05):**
1. **The OQ-1 udp broadcast bridge rule + the DHCP host pins (r9)** — `37e77fb` (feat)
2. **The DHCP gate's xtask half: assert_dhcp_dora + the dhcp sweep mode (r9)** — `e57e5f7` (feat)
3. **The DHCP fixture + the bake manifest (Harvey)** — `a37b78509` (feat)

**Task 2 (NETIP-06):**
4. **The devip ip6 surface: the icmpv6 bridge + ipifc add6/ra6/try + the format_ip fix (r9)** — `8a30e64` (feat)
5. **The v6 gate's xtask half: the ICMPv6/ND classifier + assert_nd_exchange + the v6 mode (r9)** — `8bb29a2` (feat)
6. **The v6 fixture + the finalized seam values + the deferred-items addendum (Harvey)** — `9f9a833da` (feat)

**Follow-up (review-caught):**
7. **The icmpv6 write's short-drop bound is the donor's IPICMPSZ (r9)** — `89ab75f` (fix)
8. **The dhcp fixture's rc-dialect repair (Harvey)** — `089637446` (fix — `a37b78509` had captured the pre-fix fixture; see Deviation 3)

**Plan metadata:** `41bed1534` (Harvey — this SUMMARY + STATE/ROADMAP/REQUIREMENTS) + one pending correction commit (this file's Deviation-3 attribution + the commit accounting)

## Files Created/Modified

**r9:**
- `port/src/devip/proto.rs` — the OQ-1 udp broadcast rule in `udp_write_data` (the accept-all-keyed Some(0.0.0.0) override); the icmpv6 module (icmp6_ctl/connect+announce via the shared dual-family icmp arm/icmp6_write_data/icmp6_read_data/synthesize_v6_packet/icmp6_stats + the stats counters); the dispatch wiring (proto_ctl, apply_hop_limit, status, the netlog flag arm)
- `port/src/devip/ipifc.rs` — add6 + ra6 verbs (the donor's strings + the EUI-64 pref2addr), the v6 update_ip_addrs arms in add/remove, the tentative lifecycle (try records, the confirming add pushes), the absorption extended to v6 multicast, find_local_ip's v6 arm, the status line's live sendra/recvra, the cfg(test) ether-ifc seam
- `port/src/devip.rs` — the icmpv6 clone gate + QDATA/QSTATS arms + the ICMP6 wake-slot arrays; the format_ip triple-colon fix (Rule 1 — see Deviations)
- `port/src/devip/phyadapt.rs` — StackDev::mac() (add6's pref2addr source)
- `port/src/devip_tests.rs` — 5 new pins (the absorption+netlog pin, the file-surface DORA pin, the icmpv6 round-trip pin, the headers-prefix pin, the v6-verbs pin)
- `xtask/src/pcap.rs` — the BOOTP/DHCP classifier + assert_dhcp_dora (+ 9 pins); the ICMPv6/ND classifier + assert_nd_exchange (+ 8 pins); netaddr_slirp_v6 (+ the parse pin)
- `xtask/src/main.rs` — R9_NETIP_SWEEP modes dhcp + v6 (typed fixtures, pre-spawn seam resolves, the finish arms' markers + pcap assertions, NETIP_V6_SLIRP_NA)
- `docs/devip-contract.md` — the dated 2026-11-06 notes: §12.6's file-surface predicate, §8's bridge corners, §9's verb mapping

**Harvey:**
- `build/aarch64/root-tree/lib/netip-dhcp.rc` — NEW: the NETIP-05 gate fixture
- `build/aarch64/root-tree/lib/netip-v6.rc` — NEW: the NETIP-06 gate fixture
- `build/aarch64/mkimage.sh` — the pre-bake manifest asserts both fixtures land
- `build/aarch64/netaddr.rc` — the v6 seam values finalized from the recorded probe
- `.planning/phases/08-r9-tcp-ip-path-devip-compatible-net/deferred-items.md` — the 08-07 warning addendum
- `.planning/STATE.md` / `ROADMAP.md` / `REQUIREMENTS.md` — position, progress, the decisions append, NETIP-06 complete (metadata)

## Decisions Made

- **The OQ-1 bridge predicate keys on the self-table's accept-all flag** — the addselfcache null-address side effect IS the absorbed-DHCP marker; an unspecified-laddr conv sources 0.0.0.0 only in that window, and an addressed iface keeps smoltcp's source selection (recorded in contract §12.6's dated note).
- **The icmpv6 bridge's recorded corners** (contract §8's dated note): the ICMP_USEAD6 prefix is write-side only; the wire src is smoltcp's RFC 6724 selection (no per-send override on icmp sockets — the pre-autoconf RS src=:: corner is unreachable with slaac OFF); NDISC traffic never surfaces to the socket (ipconfig's recvra sees nothing — a recorded divergence; the ND exchange runs in smoltcp's machinery, pcap-visible and snoop-fed to /net/arp); the read-synth ttl is the conv's configured ttl.
- **try records the tentative lifc devip-side and NEVER pushes it**; the confirming add pushes.  No DAD NS emission on try — the donor's own NS would land in the TX-snooped /net/arp as a WAIT entry for the tentative address, and ipconfig's DAD scan (ipv6.c:425-455) would read it as a false duplicate.
- **The slirp v6 probe verdict** (2026-11-06): slirp answers ND for fec0::2 and serves ICMPv6 echo (3/3); no unsolicited RAs observed; the guest's extra frame is its own MLDv2 join.  netaddr.rc finalized; the gate pins to the probe with a loud drift arm.
- **The DHCP fixture runs ipconfig's default mode** (dhcpwatch forks and sleeps the lease through the window; renewal is the plan's recorded out-of-scope — never soak-tested this phase).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] format_ip printed a triple colon for "::"-compression followed by more groups**
- **Found during:** Task 2 (the v6-verb pin's status assertion — the line read "fe80:::9")
- **Issue:** devip.rs's format_ip pushed a `:` separator after a "::" compression ("fec0:::2" for fec0::2) — latent since 08-04; the v4-only gates never printed that shape.  The /net/arp v6 lines, /net/ipselftab, the ipifc status, and ipconfig's DAD-scan tokenization (`cistrcmp(fields[2], buf)`) all ride this printer.
- **Fix:** the "::" now carries BOTH separators (the next group appends bare, then ":" resumes) — "fec0::2", "fe80::9", "::1".
- **Files modified:** ../r9/port/src/devip.rs
- **Verification:** the pin's status assertions pass; all 35 devip pins green (the v4-mapped shapes unchanged — they take the early dotted-quad return)
- **Committed in:** `8a30e64` (the Task-2 r9 commit)

**2. [Rule 1 - Test bug] The DORA pin asserted the Offer's length against the Discover payload's variable**
- **Found during:** Task 1's pin loop
- **Issue:** my pin computed the expected read length from the DISCOVER payload (258 bytes) while the Offer carries the shorter marker (255) — a 307-vs-310 assertion failure; the panic skipped the pin's teardown, leaving the accept-all marker set, which cascaded a failure into the (unrelated) wedge pin
- **Fix:** the Offer's payload gets its own variable; the wedge pin recovered once the assertion held (teardown runs)
- **Files modified:** ../r9/port/src/devip_tests.rs
- **Committed in:** `37e77fb` (the Task-1 r9 commit)

**3. [Rule 1 - Fixture bug] The ndb assertion used a bare `=` in a literal word**
- **Found during:** Task 1's first gate run — `rc: /lib/netip-dhcp.rc:115: token '=p': syntax error`, an uncatchable parse-time shell death (no FAIL marker; the run idled to the budget)
- **Issue:** `~ $w ip=$slirp_guestv4` — a bare `=` terminates an rc word (the 06-05 dialect lesson: "9front rc terminates words at bare '='")
- **Fix:** the `'ip='$slirp_guestv4` concatenation idiom; the fixture's header now names the discipline.  (The gate's DORA itself had already succeeded in that run — the failure was purely the fixture's parse.)
- **Files modified:** build/aarch64/root-tree/lib/netip-dhcp.rc
- **Verification:** the re-baked gate runs green (the PASS line's marker assertions include the fixture's own)
- **Committed in:** `089637446` (the follow-up fix commit — `a37b78509` had captured the pre-fix fixture; the gate's green runs rode the working tree, and the correction lands there)

**4. [Rule 2 - Missing critical] The icmpv6 write's short-drop bound was 40+4, not the donor's IPICMPSZ (40+8)**
- **Found during:** post-commit review (before any gate flow exercised it — ping writes 64 bytes)
- **Issue:** a 44..47-byte write would reach the ident overwrite at msg[4..6] out of bounds; the donor's icmpkick6 drops anything under the 40-byte IP6 area + the 8-byte icmp header area (blocklen < IPICMPSZ)
- **Fix:** both modes' guards moved to 40+8 (the v4 module's 20+8 shape), with the comment naming the donor's bound
- **Files modified:** ../r9/port/src/devip/proto.rs
- **Verification:** the full re-verify on the final tree (host suites + v6 + dhcp + loopback gates) green
- **Committed in:** `89ab75f` (the follow-up fix commit)

**5. [Rule 1 - Pin expectation] The add6 pin's expected EUI-64 address was hand-derived wrong**
- **Issue:** I expected 0x52|0x02 to flip the first octet (0x50) — but 0x52's U/L bit (0x02) is already set, so the correct derivation leaves it (fec0::5254:ff:fe12:3456 — the pin's own failure message showed the iface carrying exactly that)
- **Fix:** the pin asserts the CORRECT derived value with the reasoning in its comment
- **Committed in:** `8a30e64`

### Process notes (not code deviations)

- **smoltcp 0.14 API reality (the 08-06 class):** `Ipv6Address` is `core::net::Ipv6Addr` — construction via `From<[u8;16]>`, never `from_bytes`; `msg_type()` returns `Icmpv6Message` (`.into()` for u8).  Caught at compile time, fixed in-loop.
- **A mid-edit gate race (self-inflicted, recovered):** one DHCP gate run started while Task 2's ipifc.rs edits were mid-flight and failed at the kernel BUILD step (the half-edited tree never compiled).  No verdict was read from it; the completed module compiled and every gate re-ran on the final tree.  Lesson recorded: background gate runs only on committed trees.
- **Verify-flag precedent honored:** every boot-test invocation carried `--arch aarch64` (the 07-01/08-01 correction).

---

**Total deviations:** 5 auto-fixed (2 Rule-1 real bugs — the format_ip printer + the IPICMPSZ bound's Rule-2 robustness — and 3 in my own test/fixture code) 
**Impact on plan:** the format_ip fix is the find the phase's full-fidelity discipline exists to catch (a printer every v6 file surface tokenizes); the rest are in-loop corrections.  No locked decision changed; every prohibition holds (verified below).

## Issues Encountered

- **The gsd_run/gsd-tools shim is unavailable** in this environment (the phase's standing condition) — STATE.md/ROADMAP.md/REQUIREMENTS.md were hand-edited to the same fields the verbs maintain (position, progress count 26→27, the metrics row, the decision append, the roadmap 6/9→7/9 + the plan's checkbox, NETIP-06's checkbox + traceability row).  NETIP-05 correctly stays PENDING (08-09 also declares it — the #2388 shared-ID gate); NETIP-06 marked complete (its last declarer).
- **Concurrent-session traffic:** one docs(10) commit (`3896817e3`) landed in this plan's Harvey ledger window (disjoint planning files, no conflicts); the measured count reconciles in the frontmatter.  STATE.md's concurrent fields (the phase-10 `stopped_at` marker) were left untouched per the 08-06 discipline.
- **Pre-existing r9 warnings** (devip.rs's unused String import, netlog's flags(), proto.rs's TCPSTAT_MAX_CONN/MAX_SEGMENT — verified present at this plan's base) logged to the phase's deferred-items.md per the scope boundary (the 08-07 addendum on the existing entry).

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **08-08 unblocked (aan + the tcpblast soak machinery):** the tcp module rides the ether core via the conv CoreSel; the RX-IRQ wake leg carries the ether-leg blocking paths; both address-configuration paths (static + DHCP) now self-provision the guest the soak rides
- **08-09 unblocked (evidence + CI + push-before-pin):** the phase's r9 series (through `89ab75f`) accumulates locally on r9 main per the phase convention — the push + flake.lock advance are 08-09's scope; NETIP-05's shared-ID gate closes when 08-09 lands
- **Phase 9's venue:** the tap-leg truth verdict re-runs BOTH gates (D-15's joint acceptance) — the NOT-L2-TRUTH labels are the record's honesty; the probe-pinned slirp behavior (echo served, ND answered, no RAs) is recorded for the tap comparison
- r9's pre-existing uncommitted `aarch64/src/exec.rs` modification and the pre-existing stash entry were never touched, staged, or committed anywhere in this plan
- No blockers

## Self-Check: PASSED

- r9 commits `37e77fb`, `e57e5f7`, `8a30e64`, `8bb29a2`, `89ab75f` exist on r9 main (`git log --oneline d170850..HEAD`) — FOUND
- Harvey commits `a37b78509`, `9f9a833da` exist on master — FOUND
- `cargo test -p port devip` → 35/35 on the final tree (the 2 Task-1 pins + 3 Task-2 pins among them); `cargo test -p xtask` → 69/69 (the 9 DORA pins + 8 ND pins + the v6 parse pin) — VERIFIED at execution time (the post-fix re-verify)
- `R9_NETIP_SWEEP=dhcp cargo xtask boot-test --arch aarch64 --config virtip` → PASS on the final tree — the finish arm asserts `devip: medium ether bound #l0` + `netip: dhcp ok` + `netip: NOT-L2-TRUTH slirp venue` + `netip: NETIP-DONE` and the pcap DORA assertion (the broadcast src-0.0.0.0 Discover/Request with the guest MAC, the Offer/AK from the seam gateway, the lease 10.0.2.15 == the seam's guestv4) — VERIFIED (twice: the gate run + the post-fix re-verify)
- `R9_NETIP_SWEEP=v6 ... --config virtip` → PASS on the final tree — the markers + NETIP-DONE + the pcap's 1 NS + 1 NA + 3 echo requests — VERIFIED (the probe run + the post-fix re-verify)
- The probe census (the pcap record walk): types 128×3/129×3/135/136 + the one hop-by-hop MLDv2 frame; no RA (type 134) anywhere — RECORDED in netaddr.rc + the Decisions append
- `R9_NETIP_SWEEP=static` / `tcp` / loopback virtip modes + all 8 regression profiles → PASS (the full battery) — VERIFIED
- The DHCP gate used the unmodified 08-03-baked ipconfig: `git diff 15957628b..HEAD -- sys/src/cmd/ip sys/src/libc` is EMPTY — the D-09 transparency holds — VERIFIED
- No smoltcp dhcp-socket wiring anywhere: no `socket-dhcpv4` in port/Cargo.toml's feature list, no `dhcpv4` reference in port/src — VERIFIED (grep)
- `../r9/xtask/src/pcap.rs` contains `assert_dhcp_dora` + `assert_nd_exchange`, each with host unit tests — VERIFIED
- ipifc.rs accepts add6/ra6/try with the contract keyword lists; `Config.slaac = false` stands at every core constructor — VERIFIED (the verb pin + grep)
- The icmpv6 module carries the parse-on-write/synthesize-on-read bridge + the 40-byte write-side prefix — VERIFIED (the round-trip + headers pins)
- netaddr.rc's guestv6/gwv6 are finalized with the probe recorded in-file — VERIFIED
- r9 `aarch64/src/exec.rs` pre-existing modification + the stash entry untouched — VERIFIED (git status / stash list)
- The format_ip fix: "fec0::2"/"fe80::9"/"::1" print correctly; the pin suite + the v6 gate's arp-view assertion (which tokenizes its output) green — VERIFIED
- No `10.0.2.x`/`fec0::` literal outside the seam in the new assertion paths (netaddr_slirp/netaddr_slirp_v6 derive from netaddr.rc) — VERIFIED

---
*Phase: 08-r9-tcp-ip-path-devip-compatible-net*
*Completed: 2026-11-06*
