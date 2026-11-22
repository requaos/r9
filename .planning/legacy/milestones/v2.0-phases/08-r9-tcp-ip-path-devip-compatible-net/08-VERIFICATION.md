---
phase: 08-r9-tcp-ip-path-devip-compatible-net
verified: 2026-11-06T06:51:53Z
status: passed
score: 36/36 must-haves verified
covered_files:
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-01-PLAN.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-01-SUMMARY.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-02-PLAN.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-02-SUMMARY.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-03-PLAN.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-03-SUMMARY.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-04-PLAN.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-04-SUMMARY.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-05-PLAN.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-05-SUMMARY.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-06-PLAN.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-06-SUMMARY.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-07-PLAN.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-07-SUMMARY.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-08-PLAN.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-08-SUMMARY.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-09-PLAN.md
  - .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-09-SUMMARY.md
  - build/aarch64/subset-build.sh
  - build/aarch64/mkimage.sh
  - build/aarch64/tcpblast.c
  - build/aarch64/tcpecho.c
  - build/aarch64/netaddr.rc
  - build/aarch64/root-tree/lib/netip-loopback.rc
  - build/aarch64/root-tree/lib/netip-tcp.rc
  - build/aarch64/root-tree/lib/netip-static.rc
  - build/aarch64/root-tree/lib/netip-dhcp.rc
  - build/aarch64/root-tree/lib/netip-v6.rc
  - build/aarch64/root-tree/lib/netip-aan.rc
  - build/aarch64/root-tree/lib/netip-aan-drop.rc
  - build/aarch64/root-tree/lib/netip-soak.rc
  - build/aarch64/smoke-r9-net.expect
  - nix/smoke-r9-net.sh
  - flake.nix
  - flake.lock
  - .github/workflows/r9-shell.yml
covered_digest: "v4:sha256:a33f9d0594fcf825c3de8afc062577d0db38968b82f903c3d9dc9f9eb98298dc"
behavior_unverified: 0
overrides_applied: 0
deferred:
  - truth: "NETIP-08 tap-leg truth verdict: sustained bidirectional TCP exit gate on the tap leg"
    addressed_in: "Phase 9"
    evidence: "D-15 amendment (08-CONTEXT.md): machinery + slirp soak in Phase 8, tap verdict executes in Phase 9 as the joint Phase-8/9 acceptance gate; ROADMAP Phase 9: 'this phase's tap leg is the venue for Phase 8's NETIP-08 exit gate'; REQUIREMENTS.md correctly shows NETIP-08 Pending"
  - truth: "The D-16 sustained soak at the named sustained count (4096 rounds = 256 MiB total) has never been executed — the 64-round CI-scale gate and the 8-round override passed"
    addressed_in: "Phase 9 (tap-leg execution) + ongoing dev-host discipline"
    evidence: "D-16 locks the sustained soak as documented-manual, NOT a CI leg; the exact dev-host command is committed verbatim in build/aarch64/root-tree/lib/netip-soak.rc's header and 08-EVIDENCE.md — the phase's deliverable is the documented command plus the machinery proof, both verified"
---

# Phase 08: r9 TCP/IP Path — devip-Compatible /net — Verification Report

**Phase Goal:** `/net` exists with devip-exact file semantics — loopback + null mediums FIRST so the TCP state machine is debuggable without the NIC, then arp/ethermedium bound to devether — ending with a sustained bidirectional TCP exit gate on the tap leg. **Kernel work in sibling repo `../r9`; `sys/src/9k/ip/` is a read-only semantics reference.** (Goal text amended by 08-CONTEXT.md D-15/D-16: the tap-leg TRUTH verdict is Phase 9's joint acceptance gate; Phase 8's NETIP-08 deliverable is the soak machinery proven on slirp with the honest NOT-L2-TRUTH label plus the documented-manual sustained soak command.)
**Verified:** 2026-11-06T06:51:53Z
**Status:** passed
**Re-verification:** No — initial verification

## Verification Scope (cross-repo)

Two repos verified: Harvey at `/Users/neilskinner/code/harvey` and r9 at `/Users/neilskinner/code/r9`. Repo-state anchors, all independently confirmed:

| Anchor | Claimed | Observed | Status |
|--------|---------|----------|--------|
| r9 HEAD | `19f61ed133d96fcdbfdfc5049ab2168beae512b3` | `git rev-parse HEAD` = `19f61ed1...` | ✓ |
| r9 origin/main | == HEAD (push-before-pin) | `git ls-remote origin main` = `19f61ed1...` | ✓ |
| flake.lock r9 pin | rev `19f61ed1...`, owner `requaos`, narHash `sha256-psrO31a2...` | matches (rev + owner confirmed; narHash `sha256-psrO31a2NWiEWGyKKdydXe0R/73ULGK/u3furd6oetQ=` present) | ✓ |
| Pin commit isolation | flake.lock ALONE in `d8080f115` | `git show --stat` = pin commit, its message records `a5b1b42..19f61ed` (30 commits) | ✓ |
| r9 pre-existing dirt | `aarch64/src/exec.rs` modified, never committed | still uncommitted, untouched — matches the recorded discipline | ✓ |
| Harvey phase files | all committed | `git status` on phase paths clean (only pre-existing unrelated dirt: troff fonts, rc/bin, .planning/config.json; two untracked transient logs gitignored by design) | ✓ |

## Goal Achievement

### Observable Truths (roadmap success criteria × plan must_haves, merged + deduplicated)

**SC1 / NETIP-01 — decision gate FINAL and recorded with the virtio provenance (08-01)**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | D-03 contract doc exists and enumerates the FULL documented devip surface with divergences flagged | ✓ VERIFIED | `../r9/docs/devip-contract.md`: 844 lines, `QID(p,c,y)` scheme at :38, 10 `man-vs-src` flags, probe chain PASS (`CONTRACT-DOC-OK` — QID/cannot-bind-null/Maxincall/unknown-control-request/hangupxmit/maxfragsize/tcpstates/accept-pool all present) |
| 2 | smoltcp 0.14.0 via crates.io pin + committed Cargo.lock, locked feature set, DHCP client socket absent (D-09) | ✓ VERIFIED | `port/Cargo.toml` `smoltcp = "0.14.0"` default-features off; `Cargo.lock` `version = "0.14.0"` registry+crates.io with checksum; `socket-dhcpv4` appears only in a comment ("no socket-dhcpv4"), zero Cargo.lock matches. **Deviation noted:** the `async` feature is enabled beyond the plan's locked list — deliberately recorded in contract §12.2 ("Feature correction (08-02)": the waker methods are `#[cfg(feature = "async")]`-gated in the 0.14.0 crate source; the feature is zero-dependency, no executor/runtime/threads). Documented, not hidden — acceptable. |
| 3 | NETIP-01 recorded in HowItWorks.md beside the virtio from-scratch record (D-04) | ✓ VERIFIED | `HowItWorks.md:248` — "Provenance (Phase 8 D-01, the NETIP-01 in-repo record — recorded together with the virtio decision above, per D-04)"; names both decisions, the deliberate divergence, supply chain, MSRV clearance, reversibility |
| 4 | r9 still builds/boots with smoltcp in tree; other targets stay green | ✓ VERIFIED | Host suites re-run by this verifier green (see Behavioral Spot-Checks); EVIDENCE records 8 boot profiles PASS at the pinned HEAD; raspi3b/raspi4b/x86_64/riscv64 unaffected (devip is virt-cfg-gated) |

**SC2 / NETIP-02 + SC3 / NETIP-03 — the event-driven seam + the #I tree + NIC-free debuggability (08-02, 08-04, 08-05)**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 5 | Seam exists host-side first: single-ownership Interface+SocketSet+adapters, driven ONLY by RX-IRQ wake / poll_at-armed one-shot deadline / file-op activity (D-05) | ✓ VERIFIED | `stack.rs` 742 lines, `drive_stack` ×9, `NET_DEADLINE` AtomicU64 :245, `net_deadline()` early-returns when disarmed (:266 — NO periodic poll); `tick_check` rides the 1 ms tick (timer.rs:124-130); `note_rx_pending()` called from the virtio IRQ path (pci.rs:930); hooks installed in main.rs:514-527. Behavioral: pins `seam_installs_and_drives_clean`, `deadline_arms_from_poll_at_across_retransmit`, `stack_debt_drains_on_next_op` all PASS (re-run by this verifier) |
| 6 | Waker bridge wakes blocked readers and re-registers after EVERY wake, pinned host-side | ✓ VERIFIED | `devip/waker.rs` 123 lines (RawWaker vtable); pins `waker_bridge_fires_on_rx_close_and_buffer_space` + `register_wake_reregister_never_loses_a_wake` PASS — the behavior-dependent lost-wakeup invariant is test-exercised, not presence-assumed |
| 7 | OQ-1 resolved by a host pin of the exact DHCP-shaped exchange BEFORE any in-guest DHCP gate, verdict recorded | ✓ VERIFIED | Pins `dhcp_shaped_exchange_sources_0_0_0_0_on_the_wire` + `dhcp_broadcast_tx_shape_through_the_devip_file_surface` PASS; verdict recorded in contract §12.6 (per EVIDENCE) — PASS-path via explicit `UdpMetadata.local_address`, no wire-layer shim |
| 8 | Exactly ONE new unsafe site (RawWaker vtable) with SAFETY comment + UNSAFE-AUDIT entry; other devip modules forbid(unsafe_code) | ✓ VERIFIED | waker.rs carries the 4 unsafe vtable fns + SAFETY docs; mounted at crate root as `devip_waker` via `#[path]` (lib.rs:36-45) precisely because the devip parent forbids unsafe; UNSAFE-AUDIT.md lines 44-52/83/171 reconcile. All other devip modules carry the forbid (grep-verified; the waker.rs "forbid" match is its own explanatory comment) |
| 9 | ping 127.0.0.1 passes in-guest over loopback through the real devip file tree, committed gate evidence (NETIP-03 leg 1) | ✓ VERIFIED | Fixture `netip-loopback.rc` committed with `netip: NETIP-DONE` marker; mediums registered loopback+null FIRST (medium.rs:2-3 documents the locked sequencing; null's contract is its exact bind error `"cannot bind null device"`); gate PASS recorded at r9 `9de69c7`; the composed smoke re-executes this leg and its retained logs PASS on both GIC gens (see Spot-Checks) |
| 10 | Concurrent clone opens allocate independent convs; ctl isolation; a wedged reader never blocks a sibling | ✓ VERIFIED | conv.rs 732 lines (`Announced` state model); pins `udp_conv_isolation_distinct_ids_and_ctl_isolation` + `udp_wedged_reader_never_blocks_a_sibling_conv` PASS — the wedged-reader invariant is test-exercised |
| 11 | udp echo round-trip over loopback (NETIP-03 leg 2), 52-byte Udphdr synthesis | ✓ VERIFIED | Fixture committed; pin `udp_headers_mode_synthesizes_the_52_byte_prefix` PASS; gate PASS at r9 `9de69c7` recorded; composed-smoke retained logs cover the leg |
| 12 | devip wired into the kernel: #I attaches via namec, pread/pwrite dispatch, block/wake hooks, net-deadline on the tick — virt-only cfg-gated | ✓ VERIFIED | namec.rs:79 `'I' => crate::devip::attach(spec)`; devip.rs 1450 lines (`DEV_IP` ×8); timer.rs:130 `port::devip::stack::tick_check(n + 1)`; main.rs hook install; config_virtip.toml `dev = ["exec", "bootchain", "ether", "ip"]` |
| 13 | Every regression profile stays green | ✓ VERIFIED | EVIDENCE regression table at `19f61ed`: 8 boot profiles PASS ×8; host suites re-run by this verifier: port 152/152, xtask 74/74 (226 tests total, 0 failed) |
| 14 | In-guest tcp/7 echo over loopback (NETIP-03 leg 3): tcpecho via unmodified libc announce/listen/accept | ✓ VERIFIED | `build/aarch64/tcpecho.c` substantive (announce `tcp!*!7`, sysfatal on error); fixture `netip-tcp.rc` has `netip: tcp echo ok`; gate PASS at r9 `560efd1`; pins `tcp_announce_builds_contract_depth_pool_and_listen_pops_oldest` + `accept_pool_demuxes_two_syns_to_two_members` PASS |
| 15 | TCP ctl semantics devip-exact: blocking connect, accept pool at contract depth, close=FIN/hangup=RST/hangupxmit=close-after-drain, keepalive mapping, checksum/tcpporthogdefense accepted-and-recorded | ✓ VERIFIED (with tracked caveat) | Pins `tcp_connect_blocks_until_established_then_connected`, `tcp_connect_rst_mid_syn_maps_econrefused`, `tcp_connect_deadline_expiry_maps_etimedout`, `tcp_close_hangup_hangupxmit_wire_shapes`, `tcp_checksum_and_porthog_are_accepted_and_recorded`, `tcp_generic_verbs_land_with_contract_errors`, `tcp_state_rename_table_is_devip_exact` all PASS. **Tracked caveat (not a gap):** CR-01 (open) — the *fd-close* path (`conv_close`) drops the FIN by removing the socket before the drive; the ctl-verb path this truth names is the pinned, working one. Recorded in 08-REVIEW/08-REVIEW-DISPOSITION as tracked follow-up; does not contradict the gate claims (the review itself confirms the gates pass with the defect present). |
| 16 | /net/log blocks for netlog output with set/clear/only verbs; flag-guarded debug prints live (D-07) | ✓ VERIFIED | netlog.rs 262 lines, verbs `set`/`clear`/`only` at :189/:198/:207; pin `netlog_set_emit_wakes_blocked_reader` PASS |
| 17 | No stats/status file reads zeros forever; gate asserts non-zero counters | ✓ VERIFIED | Pin `tcp_full_exchange_moves_the_stats_counters` PASS; the fixture's stats-honesty leg asserted; soak gate reconciled InSegs 1408 / OutSegs 1216 (EVIDENCE) |
| 18 | /net/iproute, /net/ipselftab, /net/ndb with contract formats; default route mirrors into smoltcp Routes | ✓ VERIFIED | route.rs 333 lines (longest-match :90, `add_default_ipv4_route` mirror :141); pins `iproute_longest_match_verbs_and_smoltcp_mirror` + `ndb_offset_otrunc_eio_and_vers_bumps` PASS |

**08-03 — Harvey gate payloads + the D-11 addressing seam**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 19 | Gate binaries (ipconfig/ping/udpecho/aan/tcpecho/tcpblast) build for aarch64 via phase-owned payload rules, magic/manifest asserts, NOT tier-allowlist appends (D-12) | ✓ VERIFIED | subset-build.sh phase-tagged blocks at :279-311 (`08-03` tagged); magic assert `00008c47` enforced at :439-454; mkimage.sh rodata guard at :233; tcpblast.c/tcpecho.c substantive (dial/announce/sysfatal, byte-compare integrity per the soak header) |
| 20 | One committed addressing seam (netaddr.rc); every gate derives from it; no scattered 10.0.2.x (D-11, Pitfall 9) | ✓ VERIFIED | `netaddr.rc` 42 lines, leg-prefixed keys (`lo_`/`slirp_`), v6 pair finalized from the 08-07 probe; xtask parses it via `pcap::netaddr_slirp()` → `R9_HARVEY_DIR` → `build/aarch64/netaddr.rc` (pcap.rs:125-137). Grep: no 10.0.2.x in fixtures/expect outside discipline comments; r9-side matches are host unit-test fixture strings only |
| 21 | Harvey consumers byte-unmodified in network-facing behavior (libc dial/announce/listen, ipconfig, ping, udpecho, aan) | ✓ VERIFIED | `git log --since=2026-11-04` on all six paths: EMPTY — no phase-window modifications |

**SC4 / NETIP-04, NETIP-05, NETIP-06 — ethermedium + static + DHCP + IPv6 (08-06, 08-07)**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 22 | /dev/random + /dev/urandom exist, DTB-seeded, fresh bytes per read — ipconfig's truerand() resolves (D-17) | ✓ VERIFIED | port/src/devcons.rs: `random`/`urandom` files at :158-162, qid table :209 (`0o444`); arch half in virt/devcons.rs:67-128 (DTB rng-seed + CNTFRQ jitter); pure safe Rust per the forbid discipline |
| 23 | bind ether brings up the ethermedium: netif convs 0x800/0x806/0x86dd → smoltcp Medium::Ethernet adapter; gratuitous ARP observable in host pcap | ✓ VERIFIED | phyadapt.rs 772 lines (`EtherDev`/`LoopbackDev` ×31); pins `etherdev_merges_the_three_conv_queues_in_arrival_order`, `address_class_demux_loopback_vs_ether`, `gratuitous_arp_emission_content_on_add` PASS; EtherDev Drop closes the lane convs (r9 `19f61ed`) — the unbind+rebind leak fix the composed smoke exercises twice |
| 24 | Static ipconfig assigns the seam IPv4 + default route on slirp; selftab/route/arp evidence; labeled NOT-L2-TRUTH | ✓ VERIFIED | Fixture `netip-static.rc` (`netip: static ok`, prints `netip: NOT-L2-TRUTH slirp venue` at :128); gate PASS at r9 `d170850`; arp.rs NCACHE=256 LRU + 90-byte lines (:17-20, :39); pins `arp_snoop_feeds_ok_and_wait_entries_both_directions` PASS |
| 25 | ipifc add/remove boundary values follow the contract incl. exact error strings (backstop-tier truth) | ✓ VERIFIED | Wired held-out pin `ipifc_add_remove_boundary_values_follow_the_contract` PASS (re-run by this verifier) — the backstop rule satisfied by a passing wired test, not presence |
| 26 | DHCP via the REAL unmodified ipconfig dhcpclient over /net/udp broadcast fidelity: DORA pcap-observable, lease configures the guest, /net/ndb written, not-L2-proof labeled; smoltcp dhcp socket NEVER wired (D-09) | ✓ VERIFIED | Fixture `netip-dhcp.rc` (`netip: dhcp ok`, NOT-L2-TRUTH at :136); gate PASS at r9 `89ab75f`; `grep dhcpv4/Dhcpv4 port/src` — EMPTY (unwired confirmed); Harvey ipconfig untouched (row 21); pin `ipifc_dhcp_preconfig_absorbs_with_zero_iface_addresses_and_a_netlog_note` PASS |
| 27 | IPv6 ether leg with ICMPv6/ND (D-06): devip ip6 surface, unmodified ipconfig, NS/NA pcap-observable, ICMPv6 echo with gateway, not-L2-proof labeled | ✓ VERIFIED | proto.rs `icmpv6` ×14; ipifc.rs `add6` ×10; fixture `netip-v6.rc` (`netip: v6 ok`, NOT-L2-TRUTH at :100); pins `icmpv6_full_packet_round_trip_through_the_bridge`, `icmpv6_headers_mode_prefix_shape`, `ipifc_v6_verbs_add6_ra6_try` PASS; slirp v6 probe verdict recorded (fec0::15/fec0::2, NA answered, no RAs) and netaddr.rc finalized from it |
| 28 | DHCP preconfig `add 0.0.0.0` absorbed devip-side per the OQ-1 verdict | ✓ VERIFIED | Same pin as row 26 (`ipifc_dhcp_preconfig_absorbs...`) + row 7's OQ-1 pins |

**SC5 / NETIP-07, NETIP-08 (as amended by D-15) — aan + the soak machinery (08-08)**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 29 | aan resumes an interrupted tunnel with no loss of acknowledged payloads — scripted drop-and-resume loopback gate | ✓ VERIFIED | Fixtures `netip-aan.rc` + `netip-aan-drop.rc` committed: the drop walks /net/tcp/*/status and writes `hangup` to the underlying conv's ctl (:54-56 with loud FAIL arm); xtask sweep mode `"aan"` exists (main.rs:1717); gate PASS at r9 `58b3ae4` recorded with the two pre-existing kernel fixes (fd op-ref `a81143e`, rendezvous re-verification `688ed87`) each its own commit. NIC-free venue per D-14 |
| 30 | Bidirectional TCP soak machinery exists and is proven on slirp: tcpblast × host echo endpoint, pcap both-directions + zero RSTs, guest stats reconcile, labeled NOT-L2-TRUTH (D-15) | ✓ VERIFIED | tcpblast.c substantive; `HostEcho` binds 127.0.0.1:blastport pre-qemu, joins post-run (xtask main.rs:778-800+); `assert_tcp_bidirectional` (pcap.rs:791) carries fail-closed pins — `tcp_bidirectional_golden`/`no_flow_fails`/`rst_fails`/`thin_outbound_fails` PASS (re-run by this verifier): the assertion genuinely fails when a direction is missing or RSTs appear. Gate: 64 rounds = 2 MiB each way PASS + 8-round override PASS at its own scale; NOT-L2-TRUTH printed into the gate log (netip-soak.rc:152) |
| 31 | The exact dev-host sustained-soak command documented and committed (D-16) | ✓ VERIFIED | Committed verbatim in netip-soak.rc's header AND 08-EVIDENCE.md: `cd ../r9 && R9_HARVEY_DIR=<checkout> R9_NETIP_SOAK_ROUNDS=4096 R9_NETIP_SWEEP=soak cargo xtask boot-test --arch aarch64 --config virtip` (4096 rounds = 256 MiB); `soak_rounds()` plumbing verified (main.rs:1121-1127, 1-6 digit validation) |

**08-09 — composed smoke + CI + push-before-pin + evidence**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 32 | Composed expect-gated smoke drives the user-visible flows through the PACKAGED pipeline, presence-asserted (D-13) | ✓ VERIFIED | `smoke-r9-net.expect` 403 lines: staged static→unbind→dhcp→unbind→loopback(ping+udp)→tcp/7, `stage` helper asserts presence; NOT-L2-TRUTH labels stage-asserted at :254/:281; completion marker asserted twice (typed echo + response, :370-372); pid-warning-absent/nic-bind post-passes. virtshell config carries `"ip"` (r9 `14b2f23`) so the packaged kernel ships devip |
| 33 | Hosted CI runs the short net gates; sustained soak stays documented-manual (D-16) | ✓ VERIFIED | `.github/workflows/r9-shell.yml` `net-smoke:` job at :57-66 runs `nix run .#boot-r9-net-smoke` beside byte-untouched boot-smoke. **Externally verified via GitHub API:** run 42171968459 (headSha `bfc7fd8d…`, conclusion success, BOTH jobs success) and run 42173127408 (headSha `daac7e6a…`, both jobs success) — matches EVIDENCE exactly |
| 34 | Push-before-pin executed: r9 pushed BEFORE flake.lock advances as its own commit; fresh hosted CI green at the pushed HEAD | ✓ VERIFIED | origin/main == HEAD == pin == `19f61ed1…` (all three independently confirmed); pin commit `d8080f115` is flake.lock-only; CI run 42171968459 at `bfc7fd8d` (pin+compose+CI HEAD) green |
| 35 | 08-EVIDENCE.md consolidates: every gate × r9 SHA × pin × smoke verdict × CI refs, slirp gates labeled not-L2-truth, Phase-9 handoff recorded | ✓ VERIFIED | Read in full: gate→commit mapping per plan, D-16 command, regression table at `19f61ed`, OQ-1 + v6 probe verdicts, the 5-item Phase-9 handoff (incl. the multi-waiter RX-wake BLOCKER-CLASS for the tap venue), defects ledger, and the explicit "nothing here presents slirp evidence as L2 truth" statement — all consistent with what this verifier independently found in the trees |
| 36 | (Roadmap SC5 exit-gate amendment honored) The phase does NOT claim the tap-leg verdict; NETIP-08 stays open | ✓ VERIFIED | REQUIREMENTS.md: NETIP-01..07 Complete, NETIP-08 "Pending — machinery proven on slirp (08-08/08-09); the tap-leg truth verdict is Phase 9's joint acceptance gate (D-15)"; ROADMAP phase line carries the same amendment. No artifact presents slirp as L2 truth (labels verified in 4 fixtures + smoke + EVIDENCE rows) |

**Score:** 36/36 truths verified (0 present-but-behavior-unverified)

### Code-Review Cross-Check (08-REVIEW.md: 20 findings, all dispositioned `open`)

The review's own analysis states the gates pass with the defects present because of gate shapes (single-connection fixtures, teardown-by-qemu-kill) — which means **no critical finding contradicts a claimed gate result**, and this verifier's independent checks agree:

- **CR-01 (conv_close FIN-drop):** the fd-close path is defective; the ctl-verb path the must-have names is pinned green (`tcp_close_hangup_hangupxmit_wire_shapes`). Truth 15 verified as stated; the defect is tracked follow-up. The NETIP-03/NETIP-07 SUMMARY claims are gate-pass claims, and the review confirms the gates genuinely passed.
- **CR-03 (semacquire wakeup loss):** needs ≥2 waiters + an interleaved acquirer; the phase gates ran ≤1 waiter. The 08-04 SUMMARY claims (RFMEM + semaphore pair landed, donor dupseg semantics, ping gate green) hold as stated; the latent contention defect is tracked.
- **CR-07 / multi-waiter RX-wake:** already recorded by the phase itself as BLOCKER-CLASS for the tap venue in deferred-items.md + the EVIDENCE Phase-9 handoff — honest self-reporting, not a hidden gap.
- All 20 findings are `open` in 08-REVIEW-DISPOSITION.md (verified). They are tracked work, not verification gaps; none falsifies a must-have as worded.

### Required Artifacts (three-level + data-flow)

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `../r9/docs/devip-contract.md` | Full devip surface enumeration | ✓ VERIFIED | 844 lines; all probe markers present; derives from `sys/man` × `sys/src/9k/ip/` (man-vs-src flags ×10) |
| `../r9/port/Cargo.toml` + `Cargo.lock` | smoltcp pin (D-02) | ✓ VERIFIED | 0.14.0, locked features, no dhcpv4 |
| `../r9/HowItWorks.md` | NETIP-01 provenance (D-04) | ✓ VERIFIED | Joint virtio/smoltcp record at :248 |
| `../r9/port/src/devip/stack.rs` | The D-05 seam | ✓ VERIFIED | 742 lines; wired to timer (timer.rs:130) + IRQ (pci.rs:930) + hooks (main.rs:522) |
| `../r9/port/src/devip/waker.rs` | RawWaker bridge | ✓ VERIFIED | 123 lines; the one new unsafe; audit-reconciled |
| `../r9/port/src/devip/phyadapt.rs` | LoopbackDev + EtherDev | ✓ VERIFIED | 772 lines; merge/demux pinned |
| `../r9/port/src/devip/{conv,medium,ipifc,proto,netlog,route,arp}.rs` | The devip semantics modules | ✓ VERIFIED | 732+135+965+2666+262+333+376 lines; all substantive, all contain their contract markers |
| `../r9/port/src/devip.rs` | The #I device | ✓ VERIFIED | 1450 lines; namec 'I' arm wired |
| `../r9/port/src/devip_tests.rs` | Host pins | ✓ VERIFIED | 2798 lines; 35 pins run green by this verifier |
| `build/aarch64/{tcpblast,tcpecho}.c` | Soak + echo payloads | ✓ VERIFIED | Substantive (dial/announce/sysfatal/byte-compare) |
| `build/aarch64/netaddr.rc` | D-11 seam | ✓ VERIFIED | Consumed by fixtures AND parsed by xtask via R9_HARVEY_DIR |
| `build/aarch64/root-tree/lib/netip-*.rc` (8 fixtures) | Gate fixtures | ✓ VERIFIED | All committed with their pass markers + NOT-L2-TRUTH labels on the 4 slirp legs |
| `build/aarch64/smoke-r9-net.expect` + `nix/smoke-r9-net.sh` + flake app | Composed smoke (D-13) | ✓ VERIFIED | 403 + 241 lines; `boot-r9-net-smoke` app in flake.nix:163 |
| `.github/workflows/r9-shell.yml` | net-smoke job (D-16) | ✓ VERIFIED | Job present; two hosted runs externally verified green |
| `08-EVIDENCE.md` | Phase gate index | ✓ VERIFIED | Complete and internally consistent with tree state |
| `flake.lock` | r9 pin advance | ✓ VERIFIED | rev 19f61ed1…, owner requaos |

Data-flow (Level 4): the gates' data paths are real end-to-end — guest file ops → devip conv → smoltcp SocketSet → EtherDev → devether netif convs → virtio ring (RX-IRQ wake back via note_rx_pending); gate assertions derive from netaddr.rc (no hardcoded legs); pcap assertions run against actual captured frames with fail-closed pins. No static/hollow paths found.

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| namec.rs | devip.rs | `'I' => crate::devip::attach(spec)` (:79) | ✓ WIRED |
| proto.rs (icmp/udp/tcp/icmpv6 data paths) | stack.rs | `drive_stack()` after mutating ops; waker register→block on reads | ✓ WIRED (pinned) |
| timer.rs | stack.rs | `tick_check(n+1)` on the 1 ms tick; NET_DEADLINE | ✓ WIRED |
| pci.rs (virtio_net_irq) | stack.rs | `note_rx_pending()` after rx_ingress (:930) | ✓ WIRED |
| phyadapt.rs EtherDev | netif.rs + pci.rs | conv-queue merge; `tx_post`; IRQ wake | ✓ WIRED (pinned) |
| arp.rs | phyadapt.rs | snoop on receive() AND transmit() | ✓ WIRED (pinned both directions) |
| netaddr.rc | fixtures + xtask | sourced by sweeps; parsed via R9_HARVEY_DIR | ✓ WIRED |
| netip-aan-drop.rc | /net/tcp/N/ctl | status-walk + `hangup` write | ✓ WIRED |
| netip-soak.rc | xtask HostEcho | blastport from the seam; slirp forwarding | ✓ WIRED |
| flake.lock | r9 origin/main | rev == HEAD == origin/main | ✓ WIRED |
| r9-shell.yml net-smoke | smoke-r9-net.expect | `nix run .#boot-r9-net-smoke` → nix/smoke-r9-net.sh | ✓ WIRED (CI-verified) |

### Behavioral Spot-Checks (run by this verifier, 2026-11-06)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Contract doc complete | 9-marker grep probe chain | CONTRACT-DOC-OK | ✓ PASS |
| Host pins: port suite (incl. 35-pin devip suite) | `nix develop . -c cargo test -p port` (r9) | **152/152 PASS** (89 lib + 63 integration) | ✓ PASS |
| Host pins: xtask suite (pcap classifier, bidirectional fail-closed pins) | `nix develop . -c cargo test -p xtask` (r9) | **74/74 PASS** | ✓ PASS |
| Composed smoke, gicv2+tcg retained capture | grep `NET-SMOKE: ALL STAGES PASS` 08-09-net-smoke-gicv2-tcg.log | ×2 (typed echo + response) | ✓ PASS |
| Composed smoke, gicv3+hvf retained capture | same on -gicv3-hvf.log | ×2 | ✓ PASS |
| Hosted CI run 42171968459 | GitHub API | success; boot-smoke + net-smoke both success at bfc7fd8d | ✓ PASS |
| Hosted CI run 42173127408 | GitHub API | success; both jobs at daac7e6a1 | ✓ PASS |
| In-guest xtask sweep re-run (7 virtip modes) | — | not re-run locally (multi-minute qemu TCG boots exceed the spot-check budget) | ? SKIP — covered by committed gate records + retained logs + the two externally-verified hosted CI runs of the composed equivalents |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| NETIP-01 | 08-01 | TCP implementation decision FINAL + recorded with virtio provenance | ✓ SATISFIED | HowItWorks.md:248; smoltcp 0.14.0 pin + Cargo.lock |
| NETIP-02 | 08-01/02/04/05/06 | /net tree, devip-exact; loopback+null FIRST; no network syscalls | ✓ SATISFIED | #I wired; mediums registered loopback+null first; libc unmodified. Syscall-table note: SEMACQUIRE(37)/SEMRELEASE(38) were added — NON-network libc semaphore syscalls, user-adjudicated at the 08-04 blocking-human checkpoint (option A), recorded in SUMMARY/EVIDENCE/deferred-items. The "no networking syscalls" intent and the file-tree contract hold |
| NETIP-03 | 08-03/04/05/09 | ping 127.0.0.1, udp echo, tcp/7 echo as committed expect gates | ✓ SATISFIED | Fixtures + gate records + host pins + smoke logs + CI |
| NETIP-04 | 08-03/06/09 | arp + ethermedium bound to devether; static ipconfig; guest reachable | ✓ SATISFIED | EtherDev + snoop-fed /net/arp + static gate PASS (slirp, labeled) |
| NETIP-05 | 08-02/03/07/09 | DHCP via ipconfig /net/udp broadcast fidelity (or recorded fallback) | ✓ SATISFIED | Unmodified ipconfig DORA gate PASS; dhcpv4 socket never wired (fallback never taken — the requirement's primary path) |
| NETIP-06 | 08-03/07 | devip ip6 tree + ipconfig IPv6 configuration | ✓ SATISFIED | icmpv6 bridge + add6/ra6/try + v6 gate PASS (slirp, labeled; probe-finalized seam values) |
| NETIP-07 | 08-03/08 | aan session-reliability available for cpu connections | ✓ SATISFIED | Drop-and-resume loopback gate PASS (D-14's phase-8 proof; cpu venue is Phase 12 per the locked scope) |
| NETIP-08 | 08-03/08/09 | Exit gate: sustained bidirectional TCP soak on the tap leg | ➖ DEFERRED BY DESIGN | Phase-8 deliverable (machinery + slirp proof + NOT-L2-TRUTH labels + documented-manual command) VERIFIED; the tap-leg truth verdict is Phase 9's joint acceptance gate (D-15); REQUIREMENTS.md correctly shows Pending. Not a gap — the locked design |

No orphaned requirements: all 8 NETIP IDs mapped to Phase 8 in REQUIREMENTS.md appear in plan frontmatters and are accounted for above.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| nix/smoke-r9-net.sh | 171 | `XXXXXX` | ℹ️ Not a finding | mktemp's template syntax, not a debt marker |

No TBD/FIXME/XXX debt markers in any phase-modified file (Harvey + r9 devip tree scanned). No placeholder/stub/empty-implementation patterns in the payloads or fixtures. Debt-marker gate: clean.

### Human Verification Required

None. All must-have truths carry programmatic evidence: host pins re-run by this verifier (226 tests green), committed gate fixtures with recorded PASSes, retained smoke captures on both GIC generations, and two hosted CI runs externally verified via the GitHub API. The two deliberately-unexecuted items (the tap-leg verdict; the 4096-round sustained soak at the named sustained count) are locked deferrals — Phase 9's joint acceptance gate and D-16's documented-manual dev-host discipline respectively — recorded in the `deferred` frontmatter block, not human-verification uncertainties. The D-16 soak command is committed verbatim in two places so the discipline cannot silently rot; Phase 9's handoff item 3 executes the sustained soak on tap with this machinery.

### Gaps Summary

None. 36/36 must-haves verified against the live trees at the pinned commits.

---

_Verified: 2026-11-06T06:51:53Z_
_Verifier: the agent (gsd-verifier)_
