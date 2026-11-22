---
phase: 08-r9-tcp-ip-path-devip-compatible-net
plan: 06
subsystem: infra
tags: [smoltcp, devip, ethermedium, devether, arp, snoop, gratuitous-arp, ipconfig, static, slirp, chacha20, drbg, random, virtio-net, rx-irq, plan9, aarch64, r9, no_std]

# Dependency graph
requires:
  - phase: 08-r9-tcp-ip-path-devip-compatible-net (plans 08-01..08-05)
    provides: the contract doc (docs/devip-contract.md — the mediums/arp chapters), the D-05 event-driven seam + the waker bridge, the #I device tree + conv model + ipifc loopback + icmp/udp/tcp protos + netlog/route, the baked gate payloads (ipconfig/ping) + the netaddr.rc seam + the virtip profile
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: the live devether at HEAD (the precondition's virtnet gate), the netif conv machinery + the IRQ↔syscall BusyCell handoff + the published EtherDriverOps (tx_post) + the virtio_net_irq RX retire this plan's wake rides
provides:
  - /dev/random + /dev/urandom in the guest (D-17): a pure-safe-Rust ChaCha20 DRBG in devcons (no new crate, no unsafe), seeded lazily at first read from the DTB's /chosen/rng-seed with CNTFRQ/CNTPCT/tick mixing, never erroring — ipconfig's unconditional truerand() resolves; the boot log carries `devcons: random seeded`
  - The ethermedium: `bind ether <path>` brings up the ethermedium — the EtherDev adapter over devether's netif conversations 0x800/0x806/0x86dd (arrival-order merge via the new global ingress seq stamps), smoltcp's Medium::Ethernet Device (MTU 1514 header-inclusive, software checksums), the two-core seam (the conv CoreSel + the address-class demux), and the RX-IRQ wake leg (pci.rs → note_rx_pending — a WAKE, never a poll)
  - The snoop-fed /net/arp view (arp.rs): 90-byte lines with the offset%90 gate, NCACHE=256 LRU, flush/add/del (incl. the 3-field route-mediated form), fed by the adapter's RX+TX snoop of ARP/NDP frames (smoltcp wire codecs); the gratuitous ARP on every ether address add (the donor's areg=sendgarp)
  - The static ipconfig gate green on slirp (NETIP-04): the REAL unmodified ipconfig assigns the seam IPv4 + the default route; ping to the gateway gets replies; /net/arp shows the gateway entry; the pcap carries the gratuitous ARP + the ICMP exchange; the log is labeled `netip: NOT-L2-TRUTH slirp venue` (D-10)
affects: [08-07 (the DHCP gate rides the udp module over the ether leg + the P-F absorption; ipconfig's truerand path is live), 08-08 (aan's drop-and-resume + the tcpblast soak ride the ether leg), Phase 9 (the tap-leg truth verdict re-runs this static gate), Phase 11 (the D-17 device's hardening/supersession)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as estimateTokens.
actuals:
  tokens: 52600    # (184,591-char r9 diff + 6,888-char Harvey fixture diff + ~15,000-char this SUMMARY + ~4,000-char STATE/ROADMAP/REQUIREMENTS edits) / 4 — vs estimate 70,000 (confidence: low)
  tasks: 3
  commits: 6       # r9-side 4 (945e5da, 8f753ca, 198b0fd, d170850) + Harvey-side 2 (2007024c2, 6b747a1a6); +1 pending Harvey metadata commit (this file + STATE/ROADMAP/REQUIREMENTS).  The measured Harvey ledger window aef33a8fa..3037f3d36 is 13 commits — 11 are the concurrent phase-06/10 session's (disjoint files; reconciled here per the 08-04/08-05 precedent).
plan_head_before: aef33a8fa2f46ae9b287485502fe1285873c211b   # Harvey ledger base (#3968) — 08-05's metadata commit
plan_head_after: 3037f3d365de718c2b75ce87e96d4b140741aea6    # Harvey HEAD at SUMMARY write — the metadata commit follows this write per the atomic close-out order

# Tech tracking
tech-stack:
  added: []
  patterns: [the two-core seam (one smoltcp Interface per bound ipifc; the conv records its CoreSel at socket creation; the address-class demux with the single-core fallback), the arrival-order multi-queue merge (global ingress seq stamps on the netif frame stores — the adapter pops the smallest head), the TX-side snoop feedback (the /net/arp view records our own solicitations WAIT), the staged TX frame (smoltcp's infallible consume can never fail — a full ring stages one frame for the next drive's top), the lazy-seed DRBG (seeded at first read off the DTB via the registered arch hook; the once-marker rides the ATTACH_PRINTED idiom), the seam-address-as-parameter assertion (the pcap gates take netaddr.rc values as args — host-hermetic pins + the pre-spawn resolve)]

key-files:
  created: [../r9/port/src/devip/arp.rs, build/aarch64/root-tree/lib/netip-static.rc]
  modified: [../r9/port/src/devip/phyadapt.rs, ../r9/port/src/devip/medium.rs, ../r9/port/src/devip/ipifc.rs, ../r9/port/src/devip/stack.rs, ../r9/port/src/devip/conv.rs, ../r9/port/src/devip/proto.rs, ../r9/port/src/devip/route.rs, ../r9/port/src/devip.rs, ../r9/port/src/devip_tests.rs, ../r9/port/src/netif.rs, ../r9/port/src/devether.rs, ../r9/port/src/devcons.rs, ../r9/aarch64/src/main.rs, ../r9/aarch64/src/platform/virt/devcons.rs, ../r9/aarch64/src/platform/virt/pci.rs, ../r9/xtask/src/main.rs, ../r9/xtask/src/pcap.rs, build/aarch64/root-tree/lib/netip-loopback.rc, build/aarch64/mkimage.sh]

key-decisions:
  - "08-06: the two-core seam — StackState carries the loopback core + the ether core; every conv records its CoreSel at socket creation (demux on raddr for connect, laddr for announce — wildcard → ether-when-bound); the single-core fallback keeps the loopback gates byte-identical (a socket handle is valid only in its own core's SocketSet)"
  - "08-06: the RX-IRQ leg is a WAKE, never a poll — note_rx_pending() drains the devip waiter set after the BusyCell drops (tick_check's sibling shape; the no-waiters arm sets STACK_DEBT); pci.rs's RX retire calls it cfg(dev_ip)-gated right after rx_ingress"
  - "08-06: the /net/arp view is snoop-fed at the ADAPTER on BOTH receive() and transmit() (smoltcp's neighbor cache has no accessor); the netif frame stores carry global ingress seq stamps so the three-conv merge preserves arrival order; the gratuitous ARP is crafted devip-side (wire codecs) on every ether address add (the donor's areg=sendgarp)"
  - "08-06: the D-17 minimal CSPRNG is a pure-safe-Rust ChaCha20 DRBG in devcons (the forbid holds), seeded LAZILY at first read from the DTB's /chosen/rng-seed (the arch hook re-parses the DTB off the recorded VA), jitter-mixed (CNTFRQ^CNTPCT^ticks) at seed and per read; random ≡ urandom this era; Phase 11 supersedes"
  - "08-06: the static fixture never pre-binds ether — ipconfig's own binddevice does the clone+bind (a pre-bind would fail ipconfig's bind with 'interface already bound' under the era's per-medium singleton rule); ipconfig runs UNMODIFIED (D-09) — `ip/ipconfig -g $gw ether /net/ether0 add $guest $mask`"
  - "08-06: the xtask pcap assertions take the seam addresses as PARAMETERS (host-hermetic pins); the netaddr.rc reader resolves pre-spawn for static mode (a harness bug fails before a wasted TCG boot); no 10.0.2.x literal in the new assertions (D-11/Pitfall 9)"

patterns-established:
  - "The two-core seam: StackState { core (loopback), ether } + CoreSel on the conv + demux(addr) — every proto op reads the conv's recorded core; drive_stack drives both cores and arms the EARLIEST poll_at"
  - "The arrival-order merge: one global ingress seq counter in netif's INGRESS, stamped per frame before the demux loop; the adapter pops the smallest head across its three conv queues (head_seq peek, never a pop-compare)"
  - "The TX-side snoop: the adapter's emit path feeds the view BEFORE posting — the TX-side feedback the Phase-7 netif self-delivery gap would otherwise lose"
  - "The seam-address-as-parameter pcap assertion: assert_fns take the seam values (host-testable); main.rs resolves netaddr.rc pre-spawn (loud harness error) and at finish"

requirements-completed: [NETIP-02]  # NETIP-02's last declaring plan (08-01/02/04/05/06 all carry SUMMARYs — the #2388 shared-ID gate's ready subset); NETIP-04 stays PENDING — 08-09 also declares it and has no SUMMARY yet (the gate blocks marking until the last declarer finishes)

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "/dev/random + /dev/urandom (D-17): the DTB-seeded ChaCha20 DRBG in devcons — every read serves fresh bytes; ipconfig's unconditional truerand() resolves (the sysfatal path unreachable); the boot log carries `devcons: random seeded`"
    requirement: NETIP-04
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch aarch64 --config virtip → PASS; target/boot-test-virtip.log carries `devcons: random seeded` + the fixture's two-reads-differ leg (`netip: random ok`)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The ethermedium binding: EtherDev over netif convs 0x800/0x806/0x86dd (arrival-order merge, tx_post transmit, staged-TX retry), Medium::Ethernet + MTU 1514, the two-core seam with the address-class demux, the pci.rs RX wake"
    requirement: NETIP-02
    verification:
      - kind: unit
        ref: "cargo test -p port devip -- --nocapture → 30/30 green (the four ether-leg pins: the RX merge order, the ARP snoop OK/WAIT both directions + the file-surface 90-byte/offset-gate, the gratuitous-ARP content through the seam dispatch, the demux table; + the boundary backstop pin)"
        status: pass
      - kind: e2e
        ref: "R9_NETIP_SWEEP=static cargo xtask boot-test --arch aarch64 --config virtip → PASS; the log carries `devip: medium ether bound #l0`; the loopback + tcp modes stay green (the ether code inert until bound)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The snoop-fed /net/arp view (arp.rs): 90-byte lines + offset%90 gate + NCACHE=256 LRU + flush/add/del; the gateway entry post-ping; the static-add divergence recorded in the module header"
    requirement: NETIP-02
    verification:
      - kind: unit
        ref: "the arp-snoop pin (arp_snoop_feeds_ok_and_wait_entries_both_directions): RX reply → OK, TX solicitation → WAIT, WAIT→OK completion, the gratuitous TX → OK; the 90-byte-line/offset-gate assertions"
        status: pass
      - kind: e2e
        ref: "the static gate's `netip: arp view ok` (the /net/arp read post-ping carries the gateway entry)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The static ipconfig gate green on slirp (NETIP-04): the unmodified ipconfig assigns the seam IPv4 + the default route; ping to the gateway replies; the pcap carries the gratuitous ARP + the ICMP echo exchange; the log is labeled `netip: NOT-L2-TRUTH slirp venue` (D-10 — never re-read as L2 proof; the tap verdict is Phase 9's)"
    requirement: NETIP-04
    verification:
      - kind: unit
        ref: "cargo test -p xtask pcap -- --nocapture → 26/26 (the 5 new pins: gratuitous-ARP golden/absent, the ICMP exchange golden/no-reply/wrong-src_mac)"
        status: pass
      - kind: e2e
        ref: "R9_NETIP_SWEEP=static cargo xtask boot-test --arch aarch64 --config virtip → PASS — markers + the pcap assertions (the gratuitous ARP + 3 ICMP echo requests with replies) against THIS run's target/boot-test-virtip.pcap; all 8 regression profiles (virt virtlowecam virtexec virtexec3 virtmulti virtnotes virtpanic virtnet) green"
        status: pass
    human_judgment: false

# Metrics
duration: 54 min
completed: 2026-11-06
status: complete
---

# Phase 8 Plan 06: The D-17 Random Device + the Ethermedium + the Static Gate Summary

**/dev/random and /dev/urandom exist in the guest (D-17's minimal DTB-seeded ChaCha20 DRBG — pure safe Rust, no new crate, the forbid held), so ipconfig's unconditional truerand() resolves; the ethermedium binds Phase 7's devether as smoltcp's Device through the new EtherDev adapter (the three netif conversations merged in arrival order via the new global ingress seq stamps, TX through the registered tx_post, a one-frame staging for the full-ring corner), the /net/arp view is snoop-fed on BOTH adapter directions (OK on replies/announcements, WAIT on our own solicitations), the gratuitous ARP emits on every ether address add (the donor's areg=sendgarp, pcap-proven), and the RX-IRQ wake leg is live (pci.rs → note_rx_pending — a wake, never a poll). The static ipconfig gate is GREEN on slirp end-to-end: the REAL unmodified ipconfig assigns the seam address + the default route, ping to the gateway gets replies, /net/arp shows the gateway entry, and the log is labeled `netip: NOT-L2-TRUTH slirp venue` per D-10. NETIP-02 is complete (the shared-ID gate's last declarer); NETIP-04's slirp leg is green (08-09 still declares it — the tap verdict is Phase 9's).**

## Performance

- **Duration:** 54 min
- **Started:** 2026-11-06T00:42:53Z
- **Completed:** 2026-11-06T01:37:49Z
- **Tasks:** 3
- **Files modified:** 19 (17 r9 modified + 1 r9 created + 2 Harvey modified + 1 Harvey created — the list under key-files; plus planning records)

## Accomplishments

- **Task 1 (D-17 first, P-I's unblock):** devcons gains `random`/`urandom` (0444, read-only this era — no write surface) with QID constants and read arms; the ChaCha20 DRBG (the RFC 8439 quarter-round block function in safe Rust — a 32-byte key + 8-byte counter keystream) seeds LAZILY at first read from the DTB's `/chosen/rng-seed` via the registered arch hook (the lazy re-parse off the recorded DTB VA — the plan's "seeded at first read via fdt.rs"), mixed with CNTFRQ^CNTPCT^tick jitter at seed and on every read; the module comment records D-17's minimal scope + the Phase-11 supersession; the once-marker `devcons: random seeded` prints at the seed (the ATTACH_PRINTED idiom)
- **Task 2 (the ethermedium):** EtherDev (bind opens netif convs 0x800/0x806/0x86dd — conv_alloc + conv_connect per type, no file walk; receive merges arrival-ordered via netif's new per-frame global seq stamps; transmit posts whole frames via tx_post with the one-frame staging on a full ring; capabilities = Ethernet + MTU 1514 header-inclusive + software checksums); the snoop-fed /net/arp view (arp.rs — 90-byte lines, offset%90 gate, NCACHE=256 LRU, flush/add/del incl. the 3-field route-mediated form); the gratuitous ARP on add (wire-codec crafted); the two-core seam (CoreSel on every conv + the demux with the single-core fallback preserving the 08-04/08-05 gates byte-identically); the RX-IRQ wake (note_rx_pending + the devip.rs re-export + the cfg(dev_ip) pci.rs arm beside rx_ingress); the four new pins + the boundary backstop pin — 30/30 green
- **Task 3 (NETIP-04 on slirp):** netip-static.rc runs the REAL ipconfig unmodified (`-g $gw ether /net/ether0 add $guest $mask` per its usage — the fixture never pre-binds); xtask's static mode asserts the fixture markers + the ether-medium marker + the NOT-L2-TRUTH label + NETIP-DONE, then the pcap assertions (assert_gratuitous_arp + assert_icmp_exchange) against THIS run's capture, with the addresses derived from netaddr.rc through the R9_HARVEY_DIR channel (resolved pre-spawn); 5 new xtask pins (26/26); all 8 regression profiles + the loopback + tcp modes stay green

## Task Commits

Each task was committed atomically across the two repos:

**Task 1:**
1. **The D-17 minimal CSPRNG (r9)** — `945e5da` (feat)
2. **The loopback fixture's random-device leg (Harvey)** — `2007024c2` (feat)

**Task 2:**
3. **The ethermedium + the two-core seam + the arp view + the wake (r9)** — `8f753ca` (feat)

**Task 3:**
4. **The static gate's xtask half (r9)** — `198b0fd` (feat)
5. **The static fixture + the bake manifest (Harvey)** — `6b747a1a6` (feat)

**Follow-up (the must_haves' backstop truth):**
6. **The boundary-value pin (r9)** — `d170850` (test)

**Plan metadata:** (Harvey repo — this SUMMARY + STATE/ROADMAP/REQUIREMENTS, committed after this write per the atomic close-out order)

## Files Created/Modified

**r9:**
- `port/src/devcons.rs` — the random/urandom files + the DRBG + the hook surface + the once-marker (Task 1); `random_u64` for the ether core's Config seed (Task 2)
- `aarch64/src/platform/virt/devcons.rs` — the recorded DTB VA + the lazy rng-seed re-parse + the jitter sample (Task 1)
- `aarch64/src/main.rs` — the DTB-VA record in main9 + the hooks registration pre-exec (cfg virt) (Task 1)
- `port/src/devip/phyadapt.rs` — EtherDev + ScriptedEtherDev + craft_gratuitous_arp + the StackDev/tokens' ether arms (Task 2)
- `port/src/devip/arp.rs` — NEW: the snoop-fed /net/arp view (Task 2)
- `port/src/devip/medium.rs` — the ether registration + path validation + the ether bind + the `devip: medium ether bound #l0` marker (Task 2)
- `port/src/devip/stack.rs` — the two-core seam + demux + new_ether + both-core drive_stack + note_rx_pending + the pool's CoreSel (Task 2)
- `port/src/devip/conv.rs` — the conv's CoreSel field + the close/spawn-carried selection (Task 2)
- `port/src/devip/proto.rs` — every conv op demuxed to its recorded core (Task 2)
- `port/src/devip/ipifc.rs` — the per-medium IFC slots + the ether add arm (the ether core's update_ip_addrs + the gratuitous ARP) + the demuxed find_local_ip (Task 2)
- `port/src/devip/route.rs` — the default-route mirror lands on the ether core (Task 2)
- `port/src/devip.rs` — the arp delegation + the note_rx_pending re-export + the demuxed conv-file/listen sites (Task 2)
- `port/src/netif.rs` — the global ingress seq stamps + the head_seq peek (Task 2)
- `port/src/devether.rs` — the driver_ops() accessor (Task 2)
- `aarch64/src/platform/virt/pci.rs` — the cfg(dev_ip) note_rx_pending wake in the RX retire (Task 2)
- `port/src/devip_tests.rs` — the five new pins (Task 2 + the backstop follow-up)
- `xtask/src/main.rs` — R9_NETIP_SWEEP `static` + the pre-spawn seam resolve + the finish arm's static markers + pcap assertions (Task 3)
- `xtask/src/pcap.rs` — the Icmpv4Frame classifier + netaddr_slirp + assert_gratuitous_arp + assert_icmp_exchange + the 5 pins (Task 3)

**Harvey:**
- `build/aarch64/root-tree/lib/netip-loopback.rc` — the D-17 random-device leg (Task 1)
- `build/aarch64/root-tree/lib/netip-static.rc` — NEW: the static ipconfig gate fixture (Task 3)
- `build/aarch64/mkimage.sh` — the pre-bake manifest asserts netip-static.rc lands (Task 3)
- `.planning/STATE.md` / `ROADMAP.md` / `REQUIREMENTS.md` — position, decisions, progress, NETIP-02 complete (metadata)

## Decisions Made

- **The two-core seam with the single-core fallback:** StackState carries the loopback core (08-04's field — the 25 existing pins' `g.core` home, unchanged) + the ether core; every conv records its CoreSel at socket creation (the demux: 127/8+::1 → loopback; else → ether WHEN BOUND, else the loopback core — the loopback gates' wildcard announces and any-address connects keep their 08-04/08-05 behavior byte-for-byte). A conv's socket handle is valid only in its own core's SocketSet, so every proto op reads the recorded selection back.
- **The RX-IRQ leg is a WAKE, never a poll:** note_rx_pending() drains the devip waiter set after the BusyCell drops (tick_check's sibling shape; IRQ context takes no port mcslock — the BusyCell handoff only); the no-waiters arm sets STACK_DEBT for the next devip op's drain (the unsolicited-frame corner).
- **The snoop-fed /net/arp view at the adapter, both directions:** smoltcp 0.14 has no neighbor-cache accessor, so the view is devip-owned and fed by snooping ARP/NDP frames through smoltcp's wire codecs on receive() AND transmit() (the TX-side feedback records our own solicitations WAIT — the Phase-7 netif TX self-delivery gap is moot for smoltcp but the snoop wants TX frames). The static-add divergence (file-level success, no resolution steering) rides the module header per the contract doc.
- **The gratuitous ARP is devip-crafted, wire-codec-built:** smoltcp never sends one on update_ip_addrs (contract §10's trap); craft_gratuitous_arp builds the who-has-self request with wire::ArpPacket/EthernetFrame and the adapter emits it (TX-snooped too — the view records the announcer).
- **The D-17 device is lazy-seeded and never errors:** seeded at the first read from the DTB (the arch hook re-parses /chosen/rng-seed off the recorded VA), jitter-mixed per read; the read path returns bytes on every call (T-08-11: a consumer never falls back to a weaker path). random ≡ urandom this era; Phase 11 hardens/splits.
- **The static fixture never pre-binds ether:** ipconfig's own binddevice performs the clone + `bind ether /net/ether0` — the gate's fidelity point is the real binary's whole flow (D-09/D-12). (The plan text's pre-bind step would deadlock it — see Deviations 2.)
- **The pcap assertions take the seam addresses as parameters:** the pins are host-hermetic; main.rs resolves netaddr.rc pre-spawn (a harness bug fails before a wasted TCG boot) and at finish; no 10.0.2.x literal in the new assertions (D-11/Pitfall 9) — the 07-01-era SLIRP_* constants serve the older assertions only (untouched, out of scope).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The Task-1 verify's marker grep needs a /dev/random reader in the gate**
- **Found during:** Task 1 (the first virtip run after the kernel change)
- **Issue:** the plan's verify greps `target/boot-test-virtip.log` for `devcons: random seeded`, but the marker prints at the FIRST READ of /dev/random and NOTHING in the loopback gate read it (ping/udpecho never call truerand; the boot chain runs stacksmoke + rc) — the marker never printed, the grep could never pass
- **Fix:** the loopback fixture gains a small random-device leg ahead of the network legs: two 16-byte reads asserted non-empty (wc -c) and differing (the tier-1 payload's hsha1sum — the base subset carries no hash tool; the first attempt with plain `sha1sum` failed `can't exec` — the image's sha1sum is the h-prefixed tier-1 member). The first read's seed prints the marker the verify greps
- **Files modified:** build/aarch64/root-tree/lib/netip-loopback.rc
- **Verification:** the gate log carries `devcons: random seeded` + `netip: random ok`; both reads differ
- **Committed in:** `2007024c2` (the Task-1 Harvey half)

**2. [Rule 1 - Plan-text bug] The fixture's pre-bind would fail ipconfig's own bind**
- **Found during:** Task 3 (reading ipconfig's main.c flow for the fixture)
- **Issue:** the plan's Task 3 action steps the fixture through "ipifc clone → `bind ether /net/ether0`" and THEN runs ipconfig — but ipconfig's binddevice performs exactly that bind itself, and the era's per-medium singleton rule errors a second bind with "interface already bound" → ipconfig sysfatals
- **Fix:** the fixture never pre-binds; it runs the REAL ipconfig alone (the bind + add + route all ride the unmodified binary — the gate's actual fidelity point per D-09/D-12). The ether-medium bind marker (`devip: medium ether bound #l0`) still prints on ipconfig's bind
- **Files modified:** build/aarch64/root-tree/lib/netip-static.rc (authored without the pre-bind)
- **Verification:** the static gate is green end-to-end (ipconfig completed; the markers + pcap assertions pass)
- **Committed in:** `6b747a1a6` (the Task-3 Harvey half)

**3. [Rule 1 - Compile] The aarch64 hook-registration block sat in exec_bootstrap, not main9**
- **Found during:** Task 1 (the first build — `cannot find value dtb_va in this scope`)
- **Issue:** I placed the random-hooks registration beside the other devcons hook registrations, which live in `exec_bootstrap` — but `dtb_va` is a `main9` local
- **Fix:** split — main9 records the DTB VA (where it's in scope, right after the virt fallback's shadow), exec_bootstrap registers the hooks
- **Files modified:** ../r9/aarch64/src/main.rs
- **Verification:** the kernel builds; the gate passes
- **Committed in:** `945e5da` (the Task-1 r9 commit)

### Process notes (not code deviations)

- **The smoltcp 0.14 API reality vs older memory:** `Ipv6Address` is `core::net::Ipv6Addr` (0.14 moved to core types) — `.octets()`, never `.0`; `ArpRepr` is `#[non_exhaustive]` (match with a fallback arm, never a bare let-binding). Both caught at compile time and fixed in-loop.
- **The backstop pin's addition:** the must_haves' boundary-value truth ("verification: backstop") had no venue among the plan's four named pins — the fifth pin (the contract's boundary add/remove shapes + the exact error strings + the absorption's accept-all side effect) landed as the follow-up `d170850` after Task 2's commit. In-scope per the plan's own must_haves.
- **STATE.md session fields:** the concurrent session moved `stopped_at` to "Phase 10 context gathered" mid-plan — my edit updates ONLY the phase-08 execution fields (progress count, the Current Position block's plan pointer + activity line, the metrics table, the decision append, state_head/last_updated); the user's phase-10 session marker is untouched.

---

**Total deviations:** 3 auto-fixed (1 Rule 3 blocking + 1 Rule 1 plan-text + 1 Rule 1 compile) + the sanctioned backstop-pin addition
**Impact on plan:** Every deviation keeps the plan's intent — the fixture leg serves the plan's own verify command, the pre-bind drop serves the plan's unmodified-ipconfig fidelity, the main9/exec_bootstrap split is pure placement. No locked decision changed; the prohibitions all hold (verified below).

## Issues Encountered

- **gsd_run/gsd-tools shim unavailable** (the phase's standing condition) — STATE.md/ROADMAP.md/REQUIREMENTS.md hand-edited to the same fields the verbs maintain (position, progress count, the metrics row, the decision append, the roadmap 6/9 + the plan's checkbox, NETIP-02's checkbox + traceability row). NETIP-04 correctly stays PENDING (08-09 also declares it — the #2388 shared-ID gate); NETIP-02 marked complete (its last declarer).
- **Concurrent-session traffic:** 11 phase-06/10 commits landed in this plan's Harvey ledger window (docs(06), fix(06-review), the phase-10 context captures) — disjoint files, no conflicts; the measured count reconciles in the frontmatter. STATE.md's completed_plans was 25 at my edit time (the user's session had moved completed_phases to 7 with phase 06's completion); my bump to 26 reflects this plan only.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **08-07 unblocked (the DHCP gate):** the ether leg is live, truerand() resolves, the P-F absorption is pin-proven, the udp module's broadcast fidelity is the remaining surface (the OQ-1 bridge rule: local_address from the conv's recorded laddr); ipconfig's dhcpclient path rides announce udp!*!68 + headers + the Bootp data shape
- **08-08 unblocked (aan + the soak):** the tcp module rides the ether core via the conv CoreSel; the RX-IRQ wake leg carries the ether-leg blocking paths
- **Phase 9's venue:** the tap-leg truth verdict re-runs this static gate (D-15's joint acceptance) — the NOT-L2-TRUTH label is the record's honesty
- **Phase 11's adjacency:** the D-17 device's hardening/expansion supersedes this minimal construction (its module comment records the supersession)
- r9's pre-existing uncommitted `aarch64/src/exec.rs` modification and the pre-existing stash entry were never touched, staged, or committed anywhere in this plan
- No blockers

## Self-Check: PASSED

- r9 commits `945e5da`, `8f753ca`, `198b0fd`, `d170850` exist on r9 main (`git log --oneline 560efd1..HEAD`) — FOUND
- Harvey commits `2007024c2`, `6b747a1a6` exist on master — FOUND
- `../r9/port/src/devcons.rs` FILES table contains `random`/`urandom` (0444) with QID_RANDOM/QID_URANDOM + the read arms; the DRBG is ChaCha20 in safe Rust (the module's forbid holds via lib.rs; zero `unsafe` in the file's code); the module comment records D-17 + the Phase-11 supersession; no devcap/auth code — VERIFIED (grep)
- `../r9/port/src/devip/phyadapt.rs` contains EtherDev with Medium::Ethernet + MTU 1514; receive merges the 0x800/0x806/0x86dd queues by ingress seq; transmit posts via the registered tx_post — VERIFIED
- `../r9/port/src/devip/medium.rs` accepts `bind ether` (unknown path → "unknown interface type"; second bind → "interface already bound"); the loopback/null arms are byte-identical to 08-04's — VERIFIED
- `../r9/port/src/devip/arp.rs` implements the 90-byte line format with the offset%90 gate, NCACHE=256 LRU, flush/add/del (incl. the 3-field route-mediated form); the header records the static-add divergence — VERIFIED
- `../r9/port/src/devip/ipifc.rs` creates the ether Interface at bind (devether MAC, slaac=false, random_seed from devcons's DRBG) + the ether add arm's gratuitous ARP + the address-class demux — VERIFIED
- `../r9/port/src/devip/stack.rs` contains `note_rx_pending()` (the waiter snapshot → hooks().wake() after the guard drop); `../r9/port/src/devip.rs` re-exports it; `../r9/aarch64/src/platform/virt/pci.rs` calls it cfg(dev_ip)-gated beside rx_ingress; no poll runs in IRQ context — VERIFIED
- `cargo test -p port devip -- --nocapture` → 30 passed / 0 failed (25 carried + 4 ether-leg + the backstop pin); `cargo test -p xtask pcap` → 26/26 — VERIFIED at execution time
- `cargo xtask check --verbose` + `cargo xtask clippy --verbose` → exit 0, no warnings naming this plan's files — VERIFIED
- `R9_NETIP_SWEEP=static cargo xtask boot-test --arch aarch64 --config virtip` → PASS; target/boot-test-virtip.log carries `devcons: random seeded`, `#I0: devip attached`, `devip: medium ether bound #l0`, `netip: static ok`, `netip: arp view ok`, `netip: NOT-L2-TRUTH slirp venue`, `netip: NETIP-DONE`; the negative scan 0 hits; the pcap assertions passed (the gratuitous ARP + 3 ICMP echo requests with replies) — VERIFIED
- The loopback + tcp modes and all 8 regression profiles (virt, virtlowecam, virtexec, virtexec3, virtmulti, virtnotes, virtpanic, virtnet) PASS — VERIFIED at execution time
- ipconfig + libc byte-untouched across the plan window (`git diff aef33a8fa..HEAD -- sys/src/cmd/ip sys/src/libc` empty) — the D-09/NETIP-04 transparency holds — VERIFIED
- Cargo.lock/Cargo.toml unchanged (T-08-SC: the audited closure intact; no new packages) — VERIFIED
- No 10.0.2.x literal in the new xtask assertions (the addresses derive from netaddr.rc via netaddr_slirp) — VERIFIED
- r9 `aarch64/src/exec.rs` pre-existing modification + the stash entry untouched — VERIFIED (git status / stash list)
- `build/aarch64/root-tree/lib/netip-static.rc` exists, sources /lib/netaddr.rc, and carries the `netip: NOT-L2-TRUTH slirp venue` line — VERIFIED

---
*Phase: 08-r9-tcp-ip-path-devip-compatible-net*
*Completed: 2026-11-06*
