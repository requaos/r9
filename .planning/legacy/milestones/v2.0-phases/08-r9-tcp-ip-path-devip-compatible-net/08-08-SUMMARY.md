---
phase: 08-r9-tcp-ip-path-devip-compatible-net
plan: 08
subsystem: infra
tags: [smoltcp, devip, aan, tcpblast, soak, tcp, loopback, slirp, host-echo, pcap, libthread, rendezvous, fd-group, drop-and-resume, plan9, aarch64, r9, no_std]

# Dependency graph
requires:
  - phase: 08-r9-tcp-ip-path-devip-compatible-net (plans 08-01..08-07)
    provides: the tcp module aan rides (the accept pool + hangup ctl + blocking connect), the ether leg + static/DHCP/v6 gates, the baked payloads (aan/tcpblast) + the netaddr.rc seam + the virtip profile, the sweep-mode xtask pattern this plan extends
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: the devpipe machinery (the tunnel plumbing's pipe pairs) + the devether the soak's ether leg binds
provides:
  - the NETIP-07 proof green: aan client+server over tcp loopback, a scripted hangup of the underlying conv, the session's resume proven by a second marker round-trip — NIC-free (D-14), with the unmodified 08-03-baked aan binary
  - the NETIP-08 machinery green on slirp: tcpblast exchanges TCG-scaled bidirectional rounds with the host echo endpoint (xtask's 127.0.0.1-bound listener spawned pre-qemu, T-08-14), the pcap shows both directions above the scaled threshold with zero RSTs, guest /net/tcp/stats counters reconcile — labeled not-L2-truth (D-15; the tap verdict is Phase 9's joint gate)
  - the exact dev-host sustained-soak command committed in netip-soak.rc's header (D-16: R9_NETIP_SOAK_ROUNDS=4096)
  - two kernel fixes the aan gate required: the fd op-ref pattern (pread/pwrite no longer empty the slot mid-op) and sys_rendezvous's post-wake re-verification (the stale-waiter spurious-wake class closed)
  - the fault autopsy (unresolvable() names pid+name, dumps the TrapFrame + the live proc table with parent/argv topology)
affects: [08-09 (evidence consolidation + push-before-pin; NETIP-08's shared-ID gate closes when 08-09 lands), Phase 9 (the tap-leg truth verdict re-runs THIS soak machinery — D-15's joint acceptance), Phase 12 (aan's real venue — the cpu service; the Nbuf flow-control rule is recorded in deferred-items)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as estimateTokens.
actuals:
  tokens: 24500   # (47,886-char r9 diff + 21,762-char Harvey fixture/build diff + ~19,000-char this SUMMARY + ~9,000-char STATE/ROADMAP/REQUIREMENTS/WINDOWS/deferred edits) / 4 — vs estimate 46,000 (confidence: low)
  tasks: 2
  commits: 8      # r9-side 5 (a81143e, 688ed87, a01089d, d86ed9f, 58b3ae4) + Harvey-side 2 production (c93a42705, 28b709df8) + 1 pending Harvey metadata commit (this file + STATE/ROADMAP/REQUIREMENTS/WINDOWS/deferred-items).  The measured Harvey ledger window 84af15b40..28b709df8 carries 4 commits — 2 are this plan's, 2 are the concurrent phase-10 session's docs commits (disjoint planning files; reconciled here per the 08-05/06/07 precedent).
plan_head_before: 84af15b4026f39fbb20728bf147001475d73d095   # Harvey ledger base (#3968) — 08-07's final commit
plan_head_after: 28b709df82574e40dfe5e89e2083178f8316646a    # Harvey HEAD at SUMMARY write — the metadata commit follows this write per the atomic close-out order

# Tech tracking
tech-stack:
  added: []
  patterns: [the fd op-ref pattern (fdgrp's op_ref/op_done + Chan::op_clone/drop_op_ref — the slot keeps its chan across a blocking op, the donor's fdtochan incref shape), the post-wake re-verification rule for pid-keyed flat wakes (sys_rendezvous's is_parked loop — every blocking primitive re-checks), the in-band scripted drop (the drop logic baked at /lib/netip-aan-drop.rc and invoked through the tunnel as ONE plain command line), the tunnel↔rc full-duplex pipe plumbing (one devpipe pair per side, fds 0+1 dup'd onto one end — import.c's shape), the held-open-conv rc idiom (the announced conv's ctl on a numbered fd across aan's life), the rounds-scaled pcap byte threshold]

key-files:
  created: [build/aarch64/root-tree/lib/netip-aan.rc, build/aarch64/root-tree/lib/netip-aan-drop.rc, build/aarch64/root-tree/lib/netip-soak.rc]
  modified: [../r9/xtask/src/main.rs, ../r9/xtask/src/pcap.rs, ../r9/port/src/dat.rs, ../r9/port/src/fdgrp.rs, ../r9/port/src/rendez.rs, ../r9/aarch64/src/syscall.rs, ../r9/aarch64/src/proc.rs, ../r9/aarch64/src/fault.rs, build/aarch64/mkimage.sh, .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/deferred-items.md, .planning/WINDOWS.md]

key-decisions:
  - "08-08: the fd op-ref pattern replaces the checked-out-chan take/store in pread/pwrite — the take emptied the fd slot for the op's duration, so a shared-fd-group sibling's concurrent op on the SAME fd saw 'pwrite: bad fd' and wedged; the libthread shape (aan's blocked reader proc + writer main on one netfd) is exactly that interleave, first exercised by this gate"
  - "08-08: sys_rendezvous re-verifies the pairing after EVERY sched() wake — r9's wake model is pid-keyed/flat (not channel-keyed like the donor's Rendez), so a STALE devip net-waiter registration readied a rendezvous-parked proc with NO pairing (the aan server main faulted on the resulting nil q->head); every other blocking primitive already re-checked — rendezvous was the lone gap"
  - "08-08: the scripted drop is IN-BAND — the fixture transmits one plain command line (`rc /lib/netip-aan-drop.rc <port>`) and the SERVER-side rc runs the baked walk+hangup script against its own /net/tcp; the earlier client-side walk INSIDE the held-fd block leaked payloads into the held pipe twice (an rc redirect-stack/fd-reuse corner, root cause unpinned — deferred + WINDOWS #46)"
  - "08-08: aan flow-control rule recorded (deferred-items): Nbuf=10 unacked messages, and ONLY data messages drain the window — the 8 s sync beacons carry the acked count but aan.c's fromnet skips nb=0/msg=-1 before the ack processing; a >10-line one-direction burst with a silent far end wedges the sender forever (measured)"
  - "08-08: the fault autopsy landed (r9 a01089d) — unresolvable() names the faulting process (pid+name), dumps the TrapFrame, and dumps the live proc table with parent pid + exec argv; the aan gate's wild read was diagnosed from these prints alone"
  - "08-08: the soak's host echo endpoint binds 127.0.0.1 only (T-08-14) and spawns pre-qemu / joins post-run; R9_NETIP_SOAK_ROUNDS plumbs the count into the typed fixture line (default 64 = TCG scale; the D-16 sustained command with 4096 committed verbatim in the fixture header); the pcap byte threshold scales rounds x 32 KiB / 2 so the override is honest at every scale"

patterns-established:
  - "The fd op-ref pattern: pread/pwrite hold a transient clone with no open-count effects; the slot keeps its chan across the block; the offset merges back at op end; close-during-op works (the close's dispatch wakes the blocked op to EOF)"
  - "The in-band scripted drop: the drop logic is a BAKED script invoked through the tunnel as one plain command line — no transmitted braces/backquotes, no fd-heavy client block"
  - "The fault autopsy: unresolvable() prints pid+name + the TrapFrame + the live proc table (parent+argv) — post-mortem facts at the kill, not after"

requirements-completed: [NETIP-07]  # NETIP-07's declarers (08-03, 08-08) both carry SUMMARYs — the #2388 shared-ID gate's ready subset.  NETIP-08 stays PENDING: 08-09 also declares it and has no SUMMARY yet (the gate blocks marking until the last declarer finishes)

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The NETIP-07 aan gate green on tcp loopback (D-14): the session establishes, the scripted hangup kills the underlying conn (both ends see it), the client redials, and a second marker round-trip completes the resume — with the unmodified baked aan binary, NIC-free; a 'server died' line fails the gate"
    requirement: NETIP-07
    verification:
      - kind: e2e
        ref: "R9_NETIP_SWEEP=aan cargo xtask boot-test --arch aarch64 --config virtip → PASS; target/boot-test-virtip.log carries both roundtrip markers, the 'network closed' pair at the drop, the client's redial ('reconnected to tcp!127.0.0.1!7779'), 'netip: aan drop ok', 'netip: aan ok', NETIP-DONE; no 'server died' line; the finish arm asserts both roundtrips + the drop confirmation + NETIP-DONE + the forbidden-line check"
        status: pass
    human_judgment: false
  - id: D2
    description: "The NETIP-08 machinery green on slirp (D-15): the host echo endpoint (127.0.0.1-bound, spawned pre-qemu, joined post-run) + tcpblast's bidirectional rounds + the pcap bidirectional assertion (both directions above the rounds-scaled threshold, zero RSTs) + the guest-counter reconciliation — labeled NOT-L2-TRUTH; the tap verdict is Phase 9's"
    requirement: NETIP-08
    verification:
      - kind: e2e
        ref: "R9_NETIP_SWEEP=soak cargo xtask boot-test --arch aarch64 --config virtip → PASS (2097152 bytes each way, zero RSTs, InSegs 1408/OutSegs 1216 reconcile); the R9_NETIP_SOAK_ROUNDS=8 override PASS at its own scale (262144 bytes each way)"
        status: pass
      - kind: unit
        ref: "cargo test -p xtask pcap → 48/48 (the 5 new pins: bidirectional golden, RST negative, thin-outbound negative, no-flow negative, runt skip)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The two kernel fixes the aan gate required: the fd op-ref pattern (concurrent same-fd ops across a shared fd group) and sys_rendezvous's re-verification (the stale-waiter spurious-wake class) — both with the full regression battery green"
    verification:
      - kind: unit
        ref: "cargo test -p port → 152/152 (incl. the devip 35-pin suite); cargo test -p xtask → green"
        status: pass
      - kind: e2e
        ref: "all 8 regression boot profiles (virt, virtlowecam, virtexec, virtexec3, virtmulti, virtnotes, virtpanic, virtnet) + all 6 virtip modes (loopback, tcp, static, dhcp, v6, aan) PASS on the final tree"
        status: pass
    human_judgment: false
  - id: D4
    description: "The exact dev-host sustained-soak command committed (D-16): netip-soak.rc's header carries the verbatim R9_NETIP_SOAK_ROUNDS=4096 invocation + the Phase-9 tap-verdict pointer"
    requirement: NETIP-08
    verification:
      - kind: other
        ref: "grep -q 'R9_NETIP_SOAK_ROUNDS' build/aarch64/root-tree/lib/netip-soak.rc && grep -q 'Phase 9' build/aarch64/root-tree/lib/netip-soak.rc → SOAK-COMMAND-DOCUMENTED"
        status: pass
    human_judgment: false

# Metrics
duration: 140 min
completed: 2026-11-06
status: complete
---

# Phase 8 Plan 08: The aan Drop-and-Resume Gate + the tcpblast Soak Machinery Summary

**aan's session-reliability value proposition is proven NIC-free (NETIP-07): a client+server pair over tcp loopback establishes, a scripted `hangup` kills the underlying conversation (both ends see it), the client redials and the session RESUMES with a second marker round-trip — all with the unmodified 08-03-baked aan binary.  The NETIP-08 soak machinery is green on slirp and honestly labeled: tcpblast exchanges 64 TCG-scaled bidirectional rounds (exactly 2 MiB each way) with the host echo endpoint (xtask's 127.0.0.1-bound listener, spawned pre-qemu, joined post-run — T-08-14), the pcap proves both directions with zero RSTs, the guest's /net/tcp/stats counters reconcile, and the log carries `netip: NOT-L2-TRUTH slirp venue` — the tap-leg truth verdict is Phase 9's joint acceptance gate (D-15), and the exact dev-host sustained-soak command is committed in the fixture header (D-16).  Getting aan to run surfaced and fixed two pre-existing kernel bugs: the fd checked-out-chan pattern broke concurrent same-fd ops (the libthread shape), and sys_rendezvous trusted every wake (the stale-waiter spurious-ready class) — both are fixed with the full regression battery green.**

## Performance

- **Duration:** 140 min (incl. the two kernel-blocker debugging cycles)
- **Started:** 2026-11-06T02:27:34Z
- **Completed:** 2026-11-06T04:47:25Z
- **Tasks:** 2
- **Files modified:** 11 (8 r9 modified + 3 Harvey created) + mkimage.sh + planning records

## Accomplishments

- **Task 1 (NETIP-07, the aan gate):** netip-aan.rc (the fixture: the fd-5-held announced conv across aan's life, the tunnel↔rc full-duplex pipe plumbing per side, the listener-ready spin, the two round-trips with word-wise marker assertion) + netip-aan-drop.rc (the baked server-side drop: the status walk finds the Established conv whose local is the listen port, writes `hangup`, prints `netip: aan drop ok` to the console — never the tunnel) + the xtask `aan` sweep mode (both round-trip markers + the drop confirmation + NETIP-DONE + the forbidden `server died` line check).  The gate is green with the drop REAL: the log carries both roundtrips, the `network closed` pair at the drop, the client's redial, and the resume
- **Task 2 (NETIP-08 machinery):** the HostEcho endpoint (std::net, loopback-only bind, spawned pre-qemu / joined post-run, a startup line naming the port), the R9_NETIP_SOAK_ROUNDS plumbing (pre-spawn digit validation; typed into the fixture's argv), pcap.rs's TCP/IPv4 classifier + assert_tcp_bidirectional (rounds-scaled threshold, zero-RST rule) + 5 host pins, and netip-soak.rc (the 08-06 static-ipconfig preamble, the count argv with digit validation, the stats cat + fixture-side non-zero assertion, the D-16 header carrying the verbatim sustained command + the Phase-9 pointer).  Green on slirp at both the gate scale (64 rounds) and the 8-round override
- **The two kernel fixes the gate required (recorded as deviations, each its own commit):** the fd op-ref pattern and the sys_rendezvous re-verification — plus the fault autopsy (unresolvable() now names the faulting process and dumps the frame + the live proc table with the spawn topology) that diagnosed the second one

## Task Commits

Each task was committed atomically across the two repos:

**Task 1 (the aan gate):**
1. **The fd op-ref fix (r9)** — `a81143e` (fix)
2. **The sys_rendezvous re-verification (r9)** — `688ed87` (fix)
3. **The fault autopsy observability (r9)** — `a01089d` (feat)
4. **The aan + soak sweep modes + the host echo endpoint (r9 xtask)** — `58b3ae4` (feat)
5. **The aan fixtures + the bake manifest (Harvey)** — `c93a42705` (feat)

**Task 2 (the soak machinery):**
6. **The pcap TCP classifier + assert_tcp_bidirectional + 5 pins (r9)** — `d86ed9f` (test)
7. **The soak fixture + the bake manifest assert (Harvey)** — `28b709df8` (feat)

(The xtask main.rs commit at #4 carries both sweep modes because their additions interleave the same hunks — the aan-only subsets don't compile-separate cleanly; the pcap half rides its own commit at #6.)

**Plan metadata:** (Harvey repo — this SUMMARY + STATE/ROADMAP/REQUIREMENTS + WINDOWS + deferred-items, committed after this write per the atomic close-out order)

## Files Created/Modified

**r9:**
- `port/src/dat.rs` — Chan::op_clone + drop_op_ref (the transient op-reference, no open-count bumps; devmnt's clunk_if_last at the op-ref drop)
- `port/src/fdgrp.rs` — op_ref/op_done (the slot keeps its chan; the offset merges back)
- `port/src/rendez.rs` — RendezGroup::is_parked (the pairing-truth probe)
- `aarch64/src/syscall.rs` — sys_pread/sys_pwrite ride the op-ref pair; sys_rendezvous's None arm re-verifies the pairing in a loop
- `aarch64/src/proc.rs` — fd_op_ref/fd_op_done wrappers; dump_proc_states pub(crate) + the parent/argv topology print
- `aarch64/src/fault.rs` — unresolvable() names the faulter (pid+name), dumps the TrapFrame, dumps the live table
- `xtask/src/main.rs` — R9_NETIP_SWEEP modes `aan` + `soak`, the HostEcho endpoint, the R9_NETIP_SOAK_ROUNDS plumbing, the finish arms (markers + the forbidden `server died` line + the pcap bidirectional assertion + the guest-counter reconciliation)
- `xtask/src/pcap.rs` — the TCP/IPv4 classifier, parse_pcap_tcpv4, assert_tcp_bidirectional, netaddr_blastport, stat_value pub(crate), the 5 pins

**Harvey:**
- `build/aarch64/root-tree/lib/netip-aan.rc` — NEW: the NETIP-07 gate fixture
- `build/aarch64/root-tree/lib/netip-aan-drop.rc` — NEW: the server-side scripted-drop payload
- `build/aarch64/root-tree/lib/netip-soak.rc` — NEW: the NETIP-08 machinery fixture (the D-16 header with the verbatim sustained command + the Phase-9 pointer)
- `build/aarch64/mkimage.sh` — the pre-bake manifest asserts all three land
- `.planning/phases/08-*/deferred-items.md` + `.planning/WINDOWS.md` — the rc redirect corner + the pre-existing syscall.rs imports + the aan Nbuf rule (recorded, not fixed where out of scope)

## Decisions Made

- **The fd op-ref pattern** (the donor's fdtochan incref shape) replaces take/store in pread/pwrite — the take emptied the slot mid-op and a shared-group sibling's concurrent op on the same fd saw "pwrite: bad fd"; aan's blocked reader + writer main on one netfd is exactly that interleave.  (devmnt's fid lifecycle stays correct via clunk_if_last at the op-ref drop; the op-ref never bumps devpipe/devether/devip counts.)
- **sys_rendezvous re-verifies the pairing after every sched() wake** — r9's flat pid-keyed Blocked→Ready model means a stale waiter-set registration can ready a rendezvous-parked proc with no pairing; the entry's PRESENCE is the pairing truth (present → re-block; consumed → rendval valid).  Every other blocking primitive already re-checked; rendezvous was the lone gap.
- **The scripted drop is in-band** — the drop logic is a baked script (`/lib/netip-aan-drop.rc`) invoked through the tunnel as one plain command line; the server-side rc does the walk+hangup against its own /net/tcp (the same kernel's tree).  The earlier client-side-in-the-held-fd-block shape leaked payloads into the held pipe twice (an rc redirect-stack/fd-reuse corner — root cause unpinned, recorded deferred + WINDOWS #46).
- **aan's flow-control rule** (recorded in deferred-items for Phase 12): Nbuf=10 unacked; only DATA messages drain the window (beacons carry acked but fromnet skips nb=0/msg=-1 before the ack processing); a >10-line one-direction burst with a silent far end wedges the sender forever (measured — the first 19-line transmitted drop block deadlocked exactly so).
- **The fault autopsy landed** — unresolvable() names pid+name, dumps the TrapFrame, and dumps the live proc table with parent+argv (the spawn topology distinguishes libthread proccreate children from fork+exec'd commands); the aan gate's wild read was diagnosed from these prints alone.
- **The soak threshold scales with the rounds** (rounds × 32 KiB / 2, floor 64 KiB) — the override is honest at every scale (the 8-round proof passed at 262144 bytes each way; a fixed 1 MiB threshold mis-failed it before the fix).
- **The host echo endpoint binds 127.0.0.1 only** (T-08-14) and lives only for the run's duration; the port comes from the seam.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Kernel bug] The fd checked-out-chan pattern broke aan's concurrent same-fd read+write**
- **Found during:** Task 1's first gate run (the session established, then `writen; Write failed; pwrite: bad fd` and the reconnect wedged in "waiting for netreader to die")
- **Issue:** pread/pwrite TOOK the chan out of its fd slot for the op's duration — aan's fromnet blocks in read(netfd) while its main writes netfd, so the write saw an empty slot and failed "bad fd"; the blocked read (holding its own chan ref) never observed the error and never cleared netfd
- **Fix:** the fd op-ref pattern (fdgrp's op_ref/op_done + Chan::op_clone/drop_op_ref) — the slot keeps its chan across the block, the op rides a count-free clone, the offset merges back; sys_pread/sys_pwrite switched
- **Files modified:** ../r9/port/src/dat.rs, ../r9/port/src/fdgrp.rs, ../r9/aarch64/src/proc.rs, ../r9/aarch64/src/syscall.rs
- **Verification:** the aan gate's session establishes and survives; port 152/152 + xtask green; 8/8 regression profiles green
- **Committed in:** `a81143e`

**2. [Rule 1 - Kernel bug] sys_rendezvous returned unconditionally after any wake**
- **Found during:** Task 1's gate runs post-fd-fix (the server aan's MAIN faulted on a wild EL0 read — addr 0x40 = Thread.next — in libthread's runthread, twice; diagnosed via the new fault autopsy prints + a temporary rendezdbg park/wake trace that showed pid 20 woken with NO pairing)
- **Issue:** r9's wake model is pid-keyed/flat (Blocked→Ready), not channel-keyed like the donor's Rendez; a STALE devip net-waiter registration (the server main's listen-open registration, never cleared on completion) was drained by a later tick/RX sweep and readied the rendezvous-parked proc with NO peer pairing — runthread proceeded with q->head == nil
- **Fix:** the None arm loops: our park entry still present = spurious (re-block; the entry stays for the real peer); consumed = paired (rendval valid).  rendez.rs gained is_parked
- **Files modified:** ../r9/aarch64/src/syscall.rs, ../r9/port/src/rendez.rs
- **Verification:** the aan gate runs green end-to-end; the full battery stays green
- **Committed in:** `688ed87`

**3. [Rule 2 - Missing observability] The unresolvable fault print couldn't name the faulter**
- **Found during:** Task 1's debugging (a bare pc/addr pair cannot say WHICH binary faulted — aan/rc/dd share the text-base convention)
- **Fix:** unresolvable() names pid+name, dumps the TrapFrame, and dumps the live proc table (wedgedbg's dump extended with parent pid + exec-recorded argv); dump_proc_states went pub(crate)
- **Files modified:** ../r9/aarch64/src/fault.rs, ../r9/aarch64/src/proc.rs
- **Verification:** diagnosed the aan gate's wild read from these prints alone (pid 20 name=aan argv="aan -d /net/tcp/0" parent=6)
- **Committed in:** `a01089d`

**4. [Rule 1 - Fixture design] The drop moved in-band (the rc redirect-stack/fd-reuse corner)**
- **Found during:** Task 1's gate runs (measured red twice): with the walk+hangup inside the fd-6-held block, the drop line's `echo hangup > $drop/ctl` payload AND dd's records summary BOTH leaked into the held pipe's q[1] (the junk rode the tunnel; the remote rc can't-exec'd `hangup`/`0+1` — and the first shape's drop never fired, a false-pass the marker set alone couldn't catch)
- **Fix:** the drop logic is a baked script (`/lib/netip-aan-drop.rc`) transmitted as ONE plain command line (`rc /lib/netip-aan-drop.rc <port>`); the server-side rc runs it against its own /net/tcp; its confirmation rides the CONSOLE (fd 2), never the tunnel; the fixture never mixes a held-fd block with nested file/stderr redirects
- **Files modified:** build/aarch64/root-tree/lib/netip-aan.rc, build/aarch64/root-tree/lib/netip-aan-drop.rc (new)
- **Verification:** the log carries the real drop sequence: both roundtrips, the `network closed` pair, the client's redial, `netip: aan drop ok`, `netip: aan ok`, NETIP-DONE, no `server died`
- **Committed in:** `c93a42705`

**5. [Rule 1 - Fixture design] The transmitted drop block exceeded aan's flow-control window, then rc's word cap**
- **Found during:** Task 1's iterations — the first transmitted-block form (19 lines) wedged the client's fromclient on the empty pool forever (Nbuf=10 unacked; ONLY data messages drain the window — beacons carry acked but fromnet skips nb=0/msg=-1 before the ack processing — measured); the compact 8-line form hit rc's "can't exec: string too long" on the long if-line
- **Fix:** folded into deviation 4's final shape (one plain command line — never a burst, never a long word)
- **Committed in:** `c93a42705` (part of the same fixture commit)

**6. [Rule 1 - Fixture bug] The word-wise `~` assertion shape**
- **Found during:** Task 1 (resp WAS exactly the marker but `~ $"resp" 'netip: aan roundtrip 1'` failed — the `$"resp"` trailing-quote form rides rc's dangling-quote lexer corner)
- **Fix:** the response is asserted word-wise (`~ $#resp 4`, `~ $resp(1) netip:`, `~ $resp(3) roundtrip`, `~ $resp(4) 1`) — the netip-tcp.rc `$st(1)` precedent
- **Files modified:** build/aarch64/root-tree/lib/netip-aan.rc
- **Committed in:** `c93a42705`

**7. [Rule 1 - Harness] The soak fixture's stats cat must print to the console, and the byte threshold must scale with the rounds**
- **Found during:** Task 2's first gate run (the finish arm's stat_value found no `InSegs:` line — the fixture's backquote capture swallowed the content the harness reads) and the override proof (R9_NETIP_SOAK_ROUNDS=8: the fixed 1 MiB threshold mis-failed the deliberately-small run)
- **Fix:** the fixture cats /net/tcp/stats plainly (the lines ride the capture); the threshold is rounds × 32 KiB / 2 (floor 64 KiB)
- **Files modified:** build/aarch64/root-tree/lib/netip-soak.rc, ../r9/xtask/src/main.rs
- **Verification:** the soak gate green at both scales (64 rounds: 2097152 bytes each way; 8 rounds: 262144 each way), counters reconcile
- **Committed in:** `28b709df8` + `58b3ae4`

---

**Total deviations:** 7 auto-fixed (2 pre-existing kernel bugs, 1 missing observability, 4 fixture/harness-shape corrections)
**Impact on plan:** The two kernel fixes are the phase's debugging posture paying off exactly as designed (D-07: observability first — the fault autopsy diagnosed the rendezvous bug from its own prints).  The fixture redesigns keep every plan semantic: the drop is scripted, writes `hangup` to the underlying conv discovered via the status walk, and the gate asserts BOTH roundtrips + the drop's console confirmation + no `server died`.  No locked decision changed; the NETIP-08 transparency prohibition holds (the venue label rides the gate log; the tap verdict is Phase 9's).

## Issues Encountered

- **The gsd_run/gsd-tools shim is unavailable** in this environment (the phase's standing condition) — STATE.md/ROADMAP.md/REQUIREMENTS.md hand-edited to the same fields the verbs maintain (position, progress count 27→28, the metrics row, the decision append, the roadmap 7/9→8/9 + the plan's checkbox, NETIP-07's checkbox + traceability row).  NETIP-08 correctly stays PENDING (08-09 also declares it — the #2388 shared-ID gate); NETIP-07 marked complete (its last declarer).  WINDOWS.md entry 46 appended by hand-edit (the table row + the JSON block + the count bumps — the `gsd_run windows append` verb shape).
- **Concurrent-session traffic:** two phase-10 docs commits (`e6de31082`, `2cdcfc74b`) landed in this plan's Harvey ledger window (disjoint planning files, no conflicts); the measured count reconciles in the frontmatter.  STATE.md's concurrent fields (the phase-10 `stopped_at` marker) were left untouched per the 08-06/07 discipline.
- **r9's pre-existing uncommitted `aarch64/src/exec.rs` modification and the pre-existing stash entry were never touched, staged, or committed** anywhere in this plan (verified per-commit; every r9 commit named explicit paths).
- **The wedge detector (~100 s idle dump) never fired during the rendezvous debugging** — the timerprocs' 4 s wakes keep the guest out of the fully-idle class; the fault autopsy carried the diagnosis instead.  Not a defect this plan; noted for the record.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **08-09 unblocked (evidence + CI + push-before-pin):** the phase's r9 series accumulates locally on r9 main (this plan adds `a81143e`, `688ed87`, `a01089d`, `d86ed9f`, `58b3ae4` on top of 08-07's `89ab75f`) — the push + flake.lock advance are 08-09's scope; NETIP-08's shared-ID gate closes when 08-09 lands; 08-EVIDENCE.md consolidates this plan's logs (the aan gate's drop/resume sequence + the soak's pcap/counter numbers + the D-16 command)
- **Phase 9's venue:** the tap-leg truth verdict re-runs THIS soak machinery (D-15's joint acceptance) — the fixture + the host echo endpoint + assert_tcp_bidirectional are the reusable pieces; the NOT-L2-TRUTH label is the record's honesty
- **Phase 12's adjacency:** aan's real venue (the cpu service) — the Nbuf flow-control rule + the fd op-ref + the rendezvous re-verification are the enablers (aan's proccreate shape is proven on r9 now); libthread was exercised in-guest for the first time by this gate
- No blockers

## Self-Check: PASSED

- r9 commits `a81143e`, `688ed87`, `a01089d`, `d86ed9f`, `58b3ae4` exist on r9 main (`git log --oneline 89ab75f..HEAD`) — FOUND
- Harvey commits `c93a42705`, `28b709df8` exist on master — FOUND
- `build/aarch64/root-tree/lib/netip-aan.rc` exists with the announced-conv-lifetime fd-5 hold, the in-band drop via `/lib/netip-aan-drop.rc`, both roundtrip markers, and the FAIL discipline — VERIFIED
- `build/aarch64/root-tree/lib/netip-aan-drop.rc` exists (the status walk + hangup + the console-side `netip: aan drop ok`) — VERIFIED
- `build/aarch64/root-tree/lib/netip-soak.rc` exists with the count argv + digit validation, the `cat /net/tcp/stats` console leg, and the header carrying `R9_NETIP_SOAK_ROUNDS` + the Phase-9 pointer — VERIFIED (the SOAK-COMMAND-DOCUMENTED grep passes)
- The aan binary is the unmodified 08-03 build: `git diff 84af15b40..HEAD -- sys/src/cmd/aan.c sys/src/libc sys/src/libthread` is EMPTY — VERIFIED
- `../r9/xtask/src/main.rs` contains the `aan` + `soak` modes, the HostEcho endpoint (127.0.0.1-bound, spawned pre-qemu, joined post-run), and the R9_NETIP_SOAK_ROUNDS plumbing — VERIFIED (grep)
- `../r9/xtask/src/pcap.rs` contains assert_tcp_bidirectional (both directions above the caller's threshold, zero RSTs) with host unit tests — VERIFIED (48/48 pcap pins)
- target/boot-test-virtip.log (aan mode) contains both roundtrip markers + `netip: aan drop ok` + `netip: aan ok` + NETIP-DONE; no "server died" line — VERIFIED
- target/boot-test-virtip.log (soak mode) contains `tcpblast: done 64 rounds`, `netip: soak ok`, `netip: NOT-L2-TRUTH slirp venue` — VERIFIED
- All six prior virtip modes (loopback, tcp, static, dhcp, v6 — plus aan as the seventh) + all 8 regression profiles (virt, virtlowecam, virtexec, virtexec3, virtmulti, virtnotes, virtpanic, virtnet) PASS on the final tree — VERIFIED at execution time
- `cargo xtask check --verbose` + `cargo xtask clippy --verbose` → exit 0, no warnings naming this plan's files — VERIFIED
- The host echo listener binds 127.0.0.1 only (never 0.0.0.0): `TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))` — VERIFIED (main.rs grep)
- r9 `aarch64/src/exec.rs` pre-existing modification + the stash entry untouched — VERIFIED (git status / stash list per commit)

---
*Phase: 08-r9-tcp-ip-path-devip-compatible-net*
*Completed: 2026-11-06*
