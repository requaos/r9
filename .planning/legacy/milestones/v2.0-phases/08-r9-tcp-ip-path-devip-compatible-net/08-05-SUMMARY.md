---
phase: 08-r9-tcp-ip-path-devip-compatible-net
plan: 05
subsystem: infra
tags: [smoltcp, devip, tcp, netlog, iproute, accept-pool, blocking-connect, tcpecho, loopback, plan9, aarch64, r9, no_std, tdd]

# Dependency graph
requires:
  - phase: 08-r9-tcp-ip-path-devip-compatible-net (plans 08-01..08-04)
    provides: the contract doc (docs/devip-contract.md), the D-05 event-driven seam + AcceptPool, the #I device tree + conv model + ipifc loopback + icmp/udp protos, the baked gate payloads (tcpecho) + netaddr.rc seam, RFMEM + the semaphore pair
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: the devpipe/devether block/wake hook idiom the seam rides
provides:
  - The tcp proto module (../r9/port/src/devip/proto.rs): the 11-state tcpstates rename table, the P-K blocking connect bridge (Econrefused/Etimedout mappings), the P-A accept-pool announce/listen (contract §12.3 depth 128), the P-L close/hangup/hangupxmit verbs, keepalive/ttl/tos, checksum + tcpporthogdefense accepted-and-recorded (OQ-4), the §5.2 status line, the 28-tag bridge-maintained stats (P-M), tcp_gc
  - ../r9/port/src/devip/netlog.rs (NEW): /net/log blocking read over a bounded line queue, set/clear/only with the donor's Log* bit values, the flag-gated emit + netlog_dbg! print plumbing (D-07), the `netlog: flags set <mask>` ctl-acceptance marker (D-13)
  - ../r9/port/src/devip/route.rs (NEW): the /net/iproute forest with longest-match lookup + the five verbs + the smoltcp default-route mirror
  - NETIP-03's third leg green in the committed gate: the tcp/7 echo over loopback (tcpecho via unmodified libc announce/listen/accept; the rc client's connect/write/read), with the stats-honesty + status + top-level-file assertions
affects: [08-06 (ether leg + icmpv6 + D-17 /dev/random; the tcp module's v6 corners are recorded in-code), 08-07 (the DHCP gate rides the udp module; ipconfig's route adds now mirror into smoltcp), 08-08 (aan's drop-and-resume rides the hangup ctl + the accept pool), Phase 11/12]

# Actuals (#2632) — chars/4 over the realized diff, same scale as estimateTokens.
actuals:
  tokens: 46000   # (157,647-char r9 diff + 9,190-char Harvey fixture diff + ~14,000-char this SUMMARY + ~3,000-char STATE/ROADMAP/WINDOWS/deferred edits) / 4 — vs estimate 75,000 (confidence: low)
  tasks: 2
  commits: 27     # MEASURED Harvey ledger window acb3362c1..92a914e32 (#3968) — 3 are this plan's (b947c4a13, 462712bfe, 92a914e32); the other 24 are the concurrent phase-06 session's (disjoint files; reconciled here per the 08-04 precedent).  r9-side: 4 (2b647d4, 2df471b, 6528d5b, 560efd1).  +1 pending Harvey metadata commit (this file + STATE/ROADMAP/WINDOWS/deferred-items)
plan_head_before: acb3362c1d55e707341f752dcdb456d23472b9b0   # Harvey ledger base — 08-04's metadata commit
plan_head_after: 92a914e32ba42e22b7d4312e106a20d8d2538f37    # Harvey HEAD at SUMMARY write — the metadata commit follows this write per the atomic close-out order

# Tech tracking
tech-stack:
  added: []
  patterns: [the accept pool as the no-backlog listen bridge (parked open arms EVERY pool member's recv waker; smoltcp's set_state fires recv wakers on every transition), probe → drive → re-probe → register/arm → block (the loopback-synchronous wakeup-loss-free shape — generalized from icmp/udp to tcp's connect/write paths), the on-block injection seam for host pins of blocking paths (single-threaded; the guest's BusyCell invariant can't hold across host threads), conv-lifetime fd-hold rc choreography extended to tcp, contract-doc-as-spec (the TCP chapter made executable by pins + gate)]

key-files:
  created: [../r9/port/src/devip/netlog.rs, ../r9/port/src/devip/route.rs, build/aarch64/root-tree/lib/netip-tcp.rc]
  modified: [../r9/port/src/devip/proto.rs, ../r9/port/src/devip/conv.rs, ../r9/port/src/devip/stack.rs, ../r9/port/src/devip.rs, ../r9/port/src/devip/ipifc.rs, ../r9/port/src/devip_tests.rs, ../r9/xtask/src/main.rs, build/aarch64/mkimage.sh]

key-decisions:
  - "08-05: the tcp blocking paths re-probe after EVERY drive (probe → drive → re-probe → register/arm → block) — on loopback the drive completes the handshake synchronously, and a waker fired mid-drive readies a RUNNING proc (a no-op), so without the re-probe the later block wedges (the gate-caught wakeup-loss; the icmp/udp read shape generalized to tcp's connect + write-full paths)"
  - "08-05: the accept pool IS tcp's incall queue — the listen open pops the oldest connected member and materializes a NEW conv around its socket (fs_new_call_tcp), then refills; RST-died members are pruned (T-08-09); the pool tears down at conv close / the announcer's close/hangup (never a live-socket leak)"
  - "08-05: hangupxmit ≡ close() on the wire — smoltcp's egress emits FIN only once the segment covers the TX buffer's tail, so the FIN never precedes queued data; the donor's close-vs-hangupxmit distinction is the rq flush smoltcp lacks (recorded mapping corner)"
  - "08-05: netlog carries the DONOR's Log* bit values (ip.h:485-500) — the 08-04 placeholder's ad-hoc bits replaced; the D-13 netlog-activity marker is the ctl-acceptance print (clean gate traffic produces no netlog packet lines)"
  - "08-05: the tcp host pins run SINGLE-THREADED with an on-block injection seam (the scripted answer fires from INSIDE the park, recorded by BLOCK_COUNT) — two host std threads genuinely race the seam's BusyCell fail-fast (the guest's single-CPU IRQ-masked invariant cannot hold across host threads; the first threaded pin run panicked on it)"
  - "08-05: the virtip gate consumes the Harvey-baked bootfs.paq AS-IS (the harvey: initrd channel — no auto-bake); fixture changes require a manual mkimage.sh rebake before the gate run (the stale-bake failure mode cost one debug cycle)"

patterns-established:
  - "On-block injection pins: a one-shot ON_BLOCK action + a standing BLOCK_RELAY flag run from the harness's block hook — the blocking file paths are pinned against the REAL park with zero host threads"
  - "The accept-pool listen bridge: announce builds 128 listeners; the SYN demux rides smoltcp's accepts(); the listen pop materializes + refills; hangup wakes the parked open to 'listen hungup'"
  - "Bridge-maintained stats: the counters increment at every packet transition the bridge observes (connect/accept/send/recv/abort); smoltcp-internal tags print 0 (the contract's honesty rule)"

requirements-completed: []  # NETIP-02 is shared with 08-06 (pending) and NETIP-03 with 08-09 (pending) — the shared-ID gate (#2388) blocks completion marking until the last declaring plan finishes; no REQUIREMENTS.md change this plan (the 08-02/08-04 precedent)

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The tcp proto module (accept pool + blocking connect bridge + listen/err files + close/hangup/hangupxmit + status/stats) with the tcp/7 echo green in the guest gate over loopback — NETIP-03's third leg, completing ping + udp echo + tcp echo"
    requirement: NETIP-03
    verification:
      - kind: unit
        ref: "cargo test -p port devip -- --nocapture → 25/25 green (the 9 tcp pins: state rename table, connect bridge, RST→Econrefused, deadline→Etimedout, pool announce/demux/pop/refill/not-announced, listen-hangup, close/hangup/hangupxmit wire shapes, full-exchange stats, OQ-4 verbs)"
        status: pass
      - kind: e2e
        ref: "R9_NETIP_SWEEP=tcp cargo xtask boot-test --arch aarch64 --config virtip → PASS; target/boot-test-virtip.log carries tcpecho: announced/conn, netip: tcp status Established, netip: tcp echo ok, netip: tcp stats ok, netip: tcp files ok, netlog: flags set 100, netip: NETIP-DONE; negative scan 0 hits"
        status: pass
    human_judgment: false
  - id: D2
    description: "The observability + remaining file surface: /net/log with set/clear/only + the flag word + netlog_dbg! plumbing + the ctl-acceptance marker (D-07/D-13); /net/iproute with longest-match + the five verbs + the smoltcp default-route mirror; live /net/ipselftab + /net/ndb content; the generic ctl verbs on every proto with contract error strings"
    requirement: NETIP-02
    verification:
      - kind: unit
        ref: "the four Task-2 pins green: iproute longest-match/verbs/mirror (in and out), ndb offset/OTRUNC/Eio/vers, netlog set→emit→blocked-reader-wakes, the generic-verb error strings on tcp"
        status: pass
      - kind: e2e
        ref: "the tcp-mode gate log carries netlog: flags set 100 (the D-13 marker) + netip: tcp files ok (the sweep's ndb/iproute reads); the loopback-mode gate and all 8 other boot-test profiles green (regression)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Stats honesty (P-M / the zeros-forever prohibition): the 28-tag /net/tcp/stats counters move at every bridge-observed transition; the gate asserts non-zero InSegs/OutSegs after the echo exchange"
    requirement: NETIP-03
    verification:
      - kind: unit
        ref: "tcp_full_exchange_moves_the_stats_counters (ActiveOpens/PassiveOpens/InSegs/OutSegs deltas + the 28-tag format + the §5.2 status line)"
        status: pass
      - kind: e2e
        ref: "the fixture's stats leg asserts non-zero InSegs/OutSegs → netip: tcp stats ok in the gate log"
        status: pass
    human_judgment: false

# Metrics
duration: 79 min
completed: 2026-11-06
status: complete
---

# Phase 8 Plan 05: The TCP Module + Observability Summary

**TCP landed over the proven loopback slice — the tcp proto module with the accept pool (128 listeners, contract §12.3) and the blocking connect bridge (Econrefused/Etimedout per the contract), the listen/err files, the P-L close/hangup/hangupxmit verbs, and the 28-tag bridge-maintained stats — plus the phase's observability deliverable (/net/log with the donor's flag vocabulary + the flag-guarded print plumbing + the D-13 marker) and the remaining top-level surface (/net/iproute with longest-match + the smoltcp default-route mirror, live ipselftab/ndb). The in-guest tcp/7 echo gate is green end-to-end: tcpecho (unmodified libc announce/listen/accept) answered the rc client's connect/write/read through /net/tcp, with the stats-honesty assertion proving the counters move (P-M). NETIP-03's three legs (ping, udp echo, tcp echo) are all committed gates now.**

## Performance

- **Duration:** 79 min
- **Started:** 2026-11-05T23:19:10Z
- **Completed:** 2026-11-06T00:38:04Z
- **Tasks:** 2
- **Files modified:** 10 (8 r9 + 2 Harvey) + 3 created (2 r9 modules + the Harvey fixture)

## Accomplishments

- **Task 1 (the tcp module + the gate):** proto.rs's tcp module (the rename table, the P-K connect bridge with the 120 s bridge-side deadline — smoltcp retransmits forever so Etimedout is ours — the P-A pool announce/listen, the P-L verbs, keepalive/ttl/tos, the OQ-4 accepted-and-recorded pair with their netlog notes, the §5.2 status line, the P-M stats, tcp_gc), conv.rs's pool/estab/nochecksum fields + fs_new_call_tcp + the conv_close tcp arm (graceful close() before the SocketSet removal so the peer EOFs; the pool teardown never leaks member sockets), devip.rs's clone-gate/QDATA/QSTATS/listen wiring, and the netip-tcp.rc gate fixture — green end-to-end in the guest
- **Task 2 (observability + surface):** netlog.rs's completion (the `netlog: flags set <mask>` marker + netlog_dbg! + the absorbed-add/parse-reject/OQ-4 note feeds), route.rs (longest-match + verbs + the mirror), the four new pins, and the fixture's netlog + top-level-read legs — the tcp-mode gate log carries the marker (`netlog: flags set 100` = Logtcp|Logicmp|Logudp)
- **The gate-caught kernel fix:** the tcp blocking paths' wakeup-loss wedge (the loopback drive completes the handshake synchronously; a mid-drive waker fire readies a RUNNING proc — a no-op — and the later block wedged). The probe → drive → RE-PROBE → register/arm → block shape (icmp/udp's read discipline) generalized to tcp's connect + write-full paths

## Task Commits

Each task was committed atomically across the two repos:

**Task 1:**
1. **RED: the nine tcp host pins** — `2b647d4` (test) — r9 repo
2. **GREEN: the tcp module + netlog module + wiring + xtask tcp mode** — `2df471b` (feat) — r9 repo
3. **The gate fixture + mkimage manifest** — `b947c4a13` (feat) — Harvey repo
4. **The gate-caught wakeup-loss fix** — `6528d5b` (fix) — r9 repo
5. **The fixture's tcpecho path fix** — `462712bfe` (fix) — Harvey repo

**Task 2:**
6. **route.rs + netlog completion + the four pins + xtask marker** — `560efd1` (feat) — r9 repo
7. **The fixture's netlog + top-level legs** — `92a914e32` (feat) — Harvey repo

**Plan metadata:** (Harvey repo — this SUMMARY + STATE/ROADMAP/WINDOWS + deferred-items, committed after this write per the atomic close-out order)

## TDD Gate Compliance

- Task 1 was `tdd="true"`: RED commit `2b647d4` precedes GREEN `2df471b`. The RED is a compile error (49 errors naming exactly the planned-behavior surface — `tcp_state_name`/`tcp_read_data`/`tcp_write_data`/`tcp_stats`/`TCP_STATS`/`netlog`, the `pool`/`estab`/`nochecksum` conv fields), the Rust test-first RED shape (the 08-02/08-04 precedent); tdd.md's INVALID_RED categories do not apply.
- Two REAL red-glints inside the GREEN loop, both fixed in-loop and recorded: (1) the close verb wrongly took a connected conv's socket off it (the status file reads the closing state machine until file close); (2) two pin-self bugs (a stale base cursor excluding the data frame; the pool-depth arithmetic with a pending second call). The pin caught a third real bug post-GREEN at the GATE (the wakeup-loss — fixed as `6528d5b`).
- No REFACTOR commit (nothing obvious to clean).
- `workflow.tdd_mode` is absent from config.json, so the runtime gate (`check tdd-red-evidence`) is inactive (the 08-02/08-04 note stands); the plan's commit-order text governed.

## Files Created/Modified

**r9:**
- `port/src/devip/proto.rs` — the tcp module (~840 new lines) + the ctl_announce fix + the parse-reject note
- `port/src/devip/conv.rs` — pool/estab/nochecksum fields, fs_new_call_tcp, the gc arm, the conv_close tcp arm
- `port/src/devip/stack.rs` — AcceptPool Closed-pruning + teardown + members() un-gated
- `port/src/devip.rs` — the tcp wiring + the listen pool-pop arm + the netlog/route delegation
- `port/src/devip/netlog.rs` — NEW: /net/log + the flag word + emit + the marker + netlog_dbg!
- `port/src/devip/route.rs` — NEW: the route forest + longest-match + the smoltcp mirror
- `port/src/devip/ipifc.rs` — route::add call site + the absorbed-add note
- `port/src/devip_tests.rs` — 13 new pins + the harness extensions (BLOCK_COUNT, ON_BLOCK, BLOCK_RELAY, the relay, TcpLeg fin/rst/payload)
- `xtask/src/main.rs` — R9_NETIP_SWEEP `tcp` mode + the finish arm's mode-keyed markers (incl. the netlog marker)

**Harvey:**
- `build/aarch64/root-tree/lib/netip-tcp.rc` — NEW: the tcp/7 echo gate fixture (echo + status + stats + netlog + top-level legs)
- `build/aarch64/mkimage.sh` — the pre-bake manifest asserts the fixture lands
- `.planning/phases/08-*/deferred-items.md` + `.planning/WINDOWS.md` — the two gate-discovered pre-existing kernel items (recorded, not fixed — out of scope per the deviation scope boundary)

## Decisions Made

- **The blocking paths re-probe after every drive** — the gate-caught wakeup-loss made it THE tcp blocking shape (probe → drive → re-probe → register/arm → block); the icmp/udp read discipline generalized.
- **The accept pool is tcp's incall queue** — the listen pop materializes the new conv around the popped socket and refills; RST-died members prune; teardown at close/hangup never leaks sockets.
- **hangupxmit ≡ close() on the wire** (smoltcp sequences FIN after queued TX data by construction); the donor's rq-flush distinction is unmappable — recorded corner in proto.rs.
- **netlog carries the donor's Log* bits** (the 08-04 placeholder's ad-hoc bits were replaced when the module landed); the D-13 marker is the ctl-acceptance print.
- **Single-threaded pins with the on-block seam** — host threads race the seam's BusyCell (its invariant is a guest construction); the on-block injection pins the REAL blocking paths deterministically.
- **The virtip gate consumes the Harvey-baked image as-is** — fixture changes need a manual `sh build/aarch64/mkimage.sh` rebake before the gate run (recorded for the later plans' executors).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] ctl_announce couldn't parse `announce *!7` — libc announce()'s actual write**
- **Found during:** Task 1 GREEN (reading libc announce before the fixture work)
- **Issue:** the 08-04 announce path hand-rolled its arg parse (`*` exact or atoi) — `announce *!7` (the form libc's announce writes for "tcp!*!7") atoi'd to 0 → a random port; tcpecho would never be reachable at 7
- **Fix:** ctl_announce routes through set_laddrport's announcing form (the donor's Fsstdannounce shape verbatim) — `*`, `*!7`, `7`, `127.0.0.1!7` all express; the udp leg's `*!echo` shape (random port + discovery) preserved verbatim
- **Files modified:** ../r9/port/src/devip/proto.rs
- **Verification:** the pool pin's `announce 7` binds port 7; the gate's tcpecho announced and accepted (`tcpecho: announced`/`conn`); the udp gate stays green
- **Committed in:** `2df471b` (the GREEN commit)

**2. [Rule 1 - Bug] The tcp blocking paths wedged on loopback (wakeup loss)**
- **Found during:** Task 1's first gate run (tcpecho accepted the connection; the client's write and tcpecho's read both blocked forever)
- **Issue:** my tcp_connect_wait registered → drove → blocked with no re-probe after the drive; on loopback the drive completes the handshake synchronously (the SYN loops in, the SYN-ACK loops back — Established fires mid-drive); the waker fired into PENDING_WAKES and drained onto a still-running proc (a no-op), then the connect blocked forever
- **Fix:** the icmp/udp read shape applied to the connect wait AND the write's buffer-full arm: probe → drive → RE-PROBE → register/arm → block
- **Files modified:** ../r9/port/src/devip/proto.rs
- **Verification:** the gate green end-to-end; the pins stay 25/25 green
- **Committed in:** `6528d5b`

**3. [Rule 1 - Bug] The fixture invoked `ip/tcpecho` but tcpecho bakes at /bin/tcpecho**
- **Found during:** Task 1's first gate run (after the stale-bake discovery)
- **Issue:** tcpecho is the 08-03 COMPILED payload (baked beside netblast at /bin/tcpecho), not a mkmany-installed ip/ binary; the server never started and the port-discovery spin fork+exec'd /bin/cat ~1400 times → the kernel arena exhausted (oom panic — itself recorded as a pre-existing leak class, deferred item + WINDOWS #44)
- **Fix:** the fixture invokes `tcpecho &` (its baked name)
- **Files modified:** build/aarch64/root-tree/lib/netip-tcp.rc
- **Committed in:** `462712bfe`

**4. [Rule 1 - My own new code] route.rs's pad_right truncated where C's %-4M pads**
- **Found during:** Task 2 (the iproute pin: "255.0.0.0" at %-4M read back "255.")
- **Fix:** pad_right no longer truncates (the donor's print overflows the column); the pin asserts the full mask
- **Files modified:** ../r9/port/src/devip/route.rs
- **Committed in:** `560efd1` (Task 2 commit)

**5. [Rule 1 - Test-harness design] The threaded pin shape raced the seam's BusyCell**
- **Found during:** Task 1 GREEN-loop (the threaded connect pin panicked `devip: stack contended` — two host std threads genuinely race the fail-fast cell)
- **Issue:** the seam's uncontended-cell invariant is a GUEST construction (single CPU, IRQ-masked syscalls); it cannot hold across host threads
- **Fix:** the pins run single-threaded; the scripted peer's answer fires from INSIDE the park via a one-shot ON_BLOCK action (and the full-exchange pin's standing BLOCK_RELAY makes the scripted capture device behave like the guest's loopback). The block hook's invocation IS the park proof (BLOCK_COUNT)
- **Files modified:** ../r9/port/src/devip_tests.rs
- **Verification:** 25/25 pins green, deterministic across runs
- **Committed in:** `2df471b` (the GREEN commit's final shape)

### Process notes (not code deviations)

- **The stale-bake cycle:** the first tcp gate run booted the 08-04-era bootfs.paq (the harvey: initrd channel is consumed as-is — no auto-bake), so the new fixture wasn't in the image; the failure presented as the fixture's `.` source line erroring (the script file name in rc's error prefix misdirected briefly). Rebake via `sh build/aarch64/mkimage.sh` after fixture changes — recorded as a key-decision for the later plans.
- **The namec union-create gap (pre-existing, worked around):** rc's `>` redirect is Creat-first; `Creat` on a TOP-LEVEL /net file resolves the parent to the union head (the rootfs's /net) and r9's walk_one never consults the bound #I member (namec's documented no-union-consult shape). The netlog leg uses `>>` (open-first) instead. Recorded in deferred-items.md + WINDOWS #45 — the fix (union-consulting walk_one) is a namec-core change, out of this plan's scope per the scope boundary.

---

**Total deviations:** 5 auto-fixed (4 Rule 1 bugs/my-code + 1 Rule 1 harness design) + 2 recorded out-of-scope kernel items (the arena leak class, the namec union-create gap)
**Impact on plan:** Every deviation either fixes my own/08-04's code against the donor's actual behavior or corrects the harness shape; the two kernel items are pre-existing, recorded, and worked around fixture-side. No locked decision changed; the contract doc's TCP chapter stands as written (no amendments needed — the contract called every shape this plan hit).

## Issues Encountered

- **The kernel arena exhaustion under a fork+exec spin loop** — ~1400 iterations of /bin/cat (the mis-fired port-discovery spin) drained the 4 MiB QuickFit arena (`oom: size=4160` panic). The exec staging path (proc.rs ByteBacking::Mnt) is the suspect. Recorded as deferred item + WINDOWS #44 (open) — a loop-of-execs leak class for a later phase; the gate never spins when the server starts.
- **The namec union-create gap** — above.
- **gsd_run/gsd-tools shim unavailable** (the phase's standing condition) — STATE.md/ROADMAP.md hand-edited to the same fields the verbs maintain; `requirements.mark-complete` correctly a no-op anyway (NETIP-02/03 shared with pending plans — the #2388 gate).
- **Concurrent-session traffic:** 24 phase-06 commits (incl. the phase-06 completion) landed in this plan's Harvey ledger window — disjoint files, no conflicts; the measured count reconciles in the frontmatter.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **08-06 unblocked (the ether leg):** EtherDev joins StackDev; the tcp module's v6 corners are recorded in-code (the v4-only connect/read paths error "bad ip address syntax" on v6 — the 08-06 udp6/icmpv6 leg's shape); /dev/random (D-17) lands there; the route mirror serves ipconfig's static route adds; the arp view table awaits the snoop feed
- **08-07/08-08 notes:** the DHCP gate rides the udp module + the OQ-1 bridge rule (local_address from the conv's laddr — the absorbed-0.0.0.0 marker now netlogs under `ip`); aan's drop-and-resume gate rides the hangup ctl (RST on the wire) + the accept pool
- **The gate-executor's note:** rebake (`sh build/aarch64/mkimage.sh`) after ANY root-tree fixture change before a virtip gate run
- r9's pre-existing uncommitted `aarch64/src/exec.rs` modification and the pre-existing stash entry were never touched, staged, or committed anywhere in this plan
- No blockers

## Self-Check: PASSED

- r9 commits `2b647d4`, `2df471b`, `6528d5b`, `560efd1` exist on r9 main (`git log --oneline 9de69c7..HEAD`) — FOUND
- Harvey commits `b947c4a13`, `462712bfe`, `92a914e32` exist on master — FOUND
- `../r9/port/src/devip/proto.rs` contains the tcp module: `tcpstates` rename table (tcp_state_name), the blocking connect bridge with "connection refused"/"connection timed out" mappings, the accept pool wiring, "not announced"/"listen hungup", the 28-tag stats — VERIFIED (grep)
- `../r9/port/src/devip/netlog.rs` exists with set/clear/only, the donor's flag bits, the marker print, netlog_dbg! — VERIFIED
- `../r9/port/src/devip/route.rs` exists with longest-match (mask_len), the five verbs, the smoltcp default-route mirror — VERIFIED
- `build/aarch64/root-tree/lib/netip-tcp.rc` exists and drives tcpecho + the rc client + the stats/status/netlog/top-level legs — VERIFIED
- `cargo test -p port devip -- --nocapture` → 25 passed / 0 failed; full port suite 79/79 — VERIFIED at execution time
- `cargo xtask check --verbose` + `cargo xtask clippy --verbose` exit 0, no new warnings naming this plan's files — VERIFIED
- `R9_NETIP_SWEEP=tcp cargo xtask boot-test --arch aarch64 --config virtip` → PASS; the log carries `tcpecho: announced`, `tcpecho: conn`, `netlog: flags set 100`, `netip: tcp status Established`, `netip: tcp echo ok`, `netip: tcp stats ok`, `netip: tcp files ok`, `netip: NETIP-DONE`; negative scan 0 hits (the only `unimplemented` lines are the designed syscall-10 fauth pair, harness-exempted) — VERIFIED
- The loopback-mode gate + all 8 other boot-test profiles (virt, virtexec, virtexec3, virtmulti, virtnotes, virtnet, virtlowecam, virtpanic) PASS — regression VERIFIED at execution time
- libc + tcpecho.c byte-untouched across the plan window (`git diff acb3362c1..HEAD -- sys/src/libc build/aarch64/tcpecho.c` empty) — VERIFIED
- The NETIP-02 transparency prohibition: no networking syscalls added (`git -C ../r9 diff 9de69c7..HEAD -- aarch64/src/syscall.rs` empty) — the file tree is the whole contract — VERIFIED
- Cargo.lock/Cargo.toml unchanged (T-08-SC: the audited closure intact) — VERIFIED
- r9 `aarch64/src/exec.rs` pre-existing modification + the stash entry untouched — VERIFIED (git status / stash list)

---
*Phase: 08-r9-tcp-ip-path-devip-compatible-net*
*Completed: 2026-11-06*
