---
phase: 08-r9-tcp-ip-path-devip-compatible-net
plan: 04
subsystem: infra
tags: [smoltcp, devip, icmp, udp, ping, udpecho, rfmem, semacquire, loopback, plan9, aarch64, r9, no_std]

# Dependency graph
requires:
  - phase: 08-r9-tcp-ip-path-devip-compatible-net (plans 08-01/08-02/08-03)
    provides: the contract doc (docs/devip-contract.md), the D-05 event-driven seam + its host pins, the baked gate payloads + netaddr.rc seam
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: the devether/netif device-surface precedents (clone-open, StatWriter, wake-after-drop), the block/wake hook idiom
  - phase: 04-r9-userspace-exec-harvey-r9-abi (v1.0)
    provides: the per-process root-table model + eager-copy rfork that this plan's RFMEM extends
provides:
  - The #I devip device tree wired end-to-end into the r9 kernel: attach/walk/open/stat/read/write/close with devip's own QID(p,c,y) scheme, the conv model (alloc/open/ref/close + owner collapse + closeconv reset), the medium registry (loopback binds, null errors "cannot bind null device"), the ipifc loopback leg (bind/add/remove/unbind with pre-smoltcp validation), icmp + udp proto modules, namec/syscall/main/timer wiring — all cfg(dev_ip) virt-gated
  - NETIP-03 legs 1-2 green in the committed gate: ping 127.0.0.1 and the udp echo round-trip over loopback (target/boot-test-virtip.log: netip: ping ok → netip: udp echo ok → netip: NETIP-DONE)
  - RFMEM in r9 (the checkpoint resolution, decision A): Data/Bss pages share under RFMEM with PA-keyed refcount-gated frees; RFMEM families carry the shared break (sys_brk_'s family arm births grown pages shared); the rfork bss sweep; SEMACQUIRE(37)/SEMRELEASE(38) for the contended libc lock() path
  - The NETIP-02 conv-isolation truth host-pinned (distinct ids, ctl isolation, wedged reader) + the udp headers byte-exact pin + the Fsnewcall spawn pin + the advise pin
  - The rc conv-lifetime choreography (hold the clone-opened ctl on a redirected fd across the exchange) + the no-cs port-discovery shape
affects: [08-05 (tcp proto rides the conv model + the read-side drive discipline; netlog/status/stats surface landed there per plan), 08-06 (ether leg + icmpv6), 08-07 (the DHCP gate rides the udp module), Phase 11 (the syscall-table growth is recorded here)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as estimateTokens.
actuals:
  tokens: 75000    # (255,235-char r9 diff + 27,255-char Harvey diffs + ~18,000-char this SUMMARY) / 4
  tasks: 2         # + the RFMEM continuation unit (the blocking-human decision's sanctioned work)
  commits: 9       # 4 r9 (fdff261, 4298fc0, 9cc6482, 9de69c7) + 4 Harvey (fd8a9034c, 176375239, f5bee06d6, 391fb79f8) + 1 pending Harvey metadata commit; the Harvey ledger window 499b4f00e..HEAD measures 28 (24 concurrent phase-06 commits in the window are NOT this plan's work)
plan_head_before: 499b4f00ea4e802ca55323ee9fd6cc8b4fe5420e   # Harvey ledger base (#3968) — 08-03's plan_head_after
plan_head_after: 391fb79f8611253f1bb4d29733781888a26ee4ea    # Harvey HEAD at SUMMARY write — the metadata commit follows this write per the atomic close-out order

# Tech tracking
tech-stack:
  added: []
  patterns: [RFMEM family model (SHARED_PAGES PA-keyed refcounts + rfmem_brk_set shared-break arm + rfork bss sweep), PA-keyed semaphore wait queues (RFMEM sharers contend on one queue), the D-05 leg-3 READ-side drive (probe → drive → re-probe → register/arm → block), the Fsnewcall drain machine (announced-socket → per-quad spawned conv queues), conv-lifetime fd-hold rc choreography (rc(1) `<>[4]`), no-cs service discovery via /net/udp/N/local readback]

key-files:
  created: [../r9/aarch64/lib/config_virtip.toml, build/aarch64/root-tree/lib/netip-loopback.rc]
  modified: [../r9/port/src/devip.rs, ../r9/port/src/devip/conv.rs, ../r9/port/src/devip/medium.rs, ../r9/port/src/devip/ipifc.rs, ../r9/port/src/devip/proto.rs, ../r9/port/src/devip_tests.rs, ../r9/port/src/dat.rs, ../r9/port/src/namec.rs, ../r9/aarch64/src/syscall.rs, ../r9/aarch64/src/main.rs, ../r9/aarch64/src/platform/virt/timer.rs, ../r9/aarch64/src/proc.rs, ../r9/xtask/src/main.rs]

key-decisions:
  - "RFMEM landed with the donor dupseg semantics EXTENDED to growth: the checkpoint premise 'ping does not brk_-grow post-fork' was falsified at the gate, so RFMEM families carry the shared break (sys_brk_'s family arm births grown pages shared into every member; the rfork bss sweep closes the untouched-page hole; SHARED_PAGES refcounts gate frees at the last sharer)"
  - "SEMACQUIRE(37)/SEMRELEASE(38) implemented with 9k sysproc.c semantics (PA-keyed wait queues; uninterruptible-block era limit) — the contended libc lock() path exists only with shared memory; TSEMACQUIRE(52) stays loud-None"
  - "udpecho's `echo' service name never resolves without cs (libc identtrans passes it through verbatim); the gate discovers the kernel-picked port from /net/udp/N/local — a nonzero port proves the announce completed"
  - "rc fixtures must hold a devip conv's ctl open across a multi-command exchange (closeconv resets on last close); rc's `<>[4]` redirect is the man page's own idiom"
  - "A devip read path must drive the stack before blocking (the D-05 leg-3 read side): a write's egress only loops the packet into the device; the pre-block drive's ingress moves it into the destination socket and fires its waker"

patterns-established:
  - "RFMEM: donor dupseg translated to per-proc mapping_logs — share at fork (Data/Bss), refcount (SHARED_PAGES), shared growth (families + brk_ arm), dissolve at exec/exit (free_user_mappings)"
  - "The udp RX drain: smoltcp has no per-remote demux — an announced conv's socket is drained into per-quad spawned conv queues, with the Fsnewcall wake riding wake_conv after the table guard drops"
  - "Byte-exact prefix synthesis pins (the 52-byte Udphdr) from smoltcp's UdpMetadata, v4 mapped into v6 form per the v4prefix model"

requirements-completed: []  # NETIP-02/NETIP-03 are shared with pending plans (08-05..08-09 per the phase's requirements rows) — the shared-ID gate (#2388) blocks completion marking until the last declaring plan finishes; no REQUIREMENTS.md change this plan

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The #I devip tree wired end-to-end (device surface + conv model + mediums + ipifc loopback + icmp proto + kernel wiring + virtip profile) with ping 127.0.0.1 green over loopback through the real file tree (NETIP-03 leg 1)"
    requirement: NETIP-03
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch aarch64 --config virtip → PASS; target/boot-test-virtip.log carries #I0: devip attached, devip: medium loopback bound, netip: ping ok, netip: NETIP-DONE; zero negative-scan hits"
        status: pass
      - kind: unit
        ref: "cargo test -p port devip -- --nocapture → 12 passed, 0 failed (7 seam pins + 5 udp pins); full port suite 66/66"
        status: pass
      - kind: e2e
        ref: "for p in virt virtlowecam virtexec virtexec3 virtmulti virtnotes virtpanic virtnet; boot-test --arch aarch64 → 8/8 PASS (no regression with the devip code in the tree)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The udp proto module (headers verb + 52-byte Udphdr synthesis + Fsnewcall spawn/demux + Qmsg reads + advise hangup) with the NETIP-02 conv-isolation truth host-pinned and the udp echo round-trip green in the guest gate (NETIP-03 leg 2)"
    requirement: NETIP-02
    verification:
      - kind: unit
        ref: "the five Task-2 pins green: udp_headers_mode_synthesizes_the_52_byte_prefix, udp_fsnewcall_spawns_on_new_remote_quad, udp_conv_isolation_distinct_ids_and_ctl_isolation, udp_wedged_reader_never_blocks_a_sibling_conv, udp_advise_hangup_lands_in_errq_unless_ignoreadvice"
        status: pass
      - kind: e2e
        ref: "target/boot-test-virtip.log carries netip: udp echo ok after netip: ping ok; udpecho ran unmodified (the baked 08-03 binary); no libc edits"
        status: pass
    human_judgment: false
  - id: D3
    description: "The blocking-human checkpoint resolution (decision A): RFMEM shared memory with the donor dupseg semantics (incl. growth) + the semaphore pair — ping's rfork(RFPROC|RFMEM|RFFDG) reader/sender pair works; the phase-08 deferred item and WINDOWS.md #43 resolved"
    requirement: NETIP-03
    verification:
      - kind: unit
        ref: "proc::rfmem_tests — 6 pins green (share/release/born-shared/double-fork/independence/reshare lifecycle); cargo test --package aarch64 --bins 14/14"
        status: pass
      - kind: e2e
        ref: "the exec-family profiles (virtexec, virtexec3, virtmulti, virtnotes) green with RFMEM in the tree; the virtip ping gate (the RFMEM consumer) green"
        status: pass
    human_judgment: false

# Metrics
duration: 95 min across two segments (incl. the blocking-human decision wait)
completed: 2026-11-05
status: complete
---

# Phase 8 Plan 04: The devip TRACER — ping + udp echo over loopback, and RFMEM Summary

**The phase's tracer landed end-to-end: the #I devip file tree wired through the r9 kernel (conv model, mediums, ipifc loopback, icmp+udp protos over the D-05 seam), with `ping 127.0.0.1` AND the udp echo round-trip green in the committed virtip gate — and the checkpoint's sanctioned continuation: RFMEM shared memory (donor dupseg semantics extended to shared bss growth via RFMEM families) plus the SEMACQUIRE/SEMRELEASE pair that libc's contended lock() needs under it.**

## Performance

- **Duration:** ~95 min across two segments (the pre-checkpoint Task 1 segment, the blocking-human decision wait, and the continuation segment)
- **Started:** 2026-11-05T19:40:00Z (pre-checkpoint segment; the RFMEM gap checkpoint returned 2026-11-05T19:54:26Z)
- **Completed:** 2026-11-05T23:15:35Z
- **Tasks:** 2 (+ the RFMEM continuation unit the checkpoint sanctioned)
- **Files modified:** 16 (13 r9 + the new config_virtip.toml + the Harvey fixture + planning records)

## Accomplishments

- **Task 1 (the tracer, pre-checkpoint segment):** the #I device surface (devip.rs — devip's own `QID(p,c,y)` scheme, the three-level tree, clone-allocates, owner collapse, StatWriter records), the conv model (conv.rs — five states, ref/closeconv, waiter sets), the medium registry ("loopback" binds, "null" errors exactly `cannot bind null device`), the ipifc loopback leg with pre-smoltcp validation (the P-F absorption rule), the icmp proto (lport-is-ident overwrite, blocking connect/read per the P-K bridge + the D-05 legs), the full kernel wiring (dat.rs DEV_IP=9, the namec/syscall/main/timer arms with drain_debt_if_set first), the virtip profile + xtask arm, and the Harvey gate fixture
- **The checkpoint + continuation (this segment):** the tracer's ping leg wedged on r9's unhonored RFMEM (ping's reader/sender pair shares its request list); the blocking-human decision (option A — implement now) resolved it. RFMEM landed with the donor's dupseg shape — Data/Bss share (same PA in both procs' root tables), Stack/Tos/Text fresh copies, refcount-gated frees at the last sharer — EXTENDED to the donor's shared-segment growth when the gate falsified the checkpoint's no-post-fork-growth premise: RFMEM families carry the shared break (sys_brk_'s family arm births grown pages shared into every member; the rfork bss sweep closes the untouched-page hole). SEMACQUIRE(37)/SEMRELEASE(38) landed with it (the contended libc lock() path exists only under shared memory). The ping gate then passed.
- **Task 2 (udp expansion):** the udp module (headers=7 with byte-exact 52-byte Udphdr synthesis from UdpMetadata, announce/connect socket binding, the Fsnewcall spawn/demux machine over a drain, Qmsg reads, the advise hangup wiring per contract §12.7, the four-field stats), the five host pins (headers byte-exact, spawn+listen pop, distinct ids + ctl isolation, wedged reader, advise), the fixture's udp leg (conv-lifetime fd-hold choreography + no-cs port discovery), and the xtask finish marker
- **Regression:** all 8 pre-existing boot-test profiles green with everything in the tree; port suite 66/66; check + clippy clean (no new warnings)

## Task Commits

Each task was committed atomically across the two repos:

**Pre-checkpoint segment (verified present at continuation start):**
1. **Task 1 implementation (r9):** `fdff261` — the #I devip device tree + kernel wiring + the virtip profile
2. **Task 1 Harvey half:** `fd8a9034c` — the netip-loopback gate fixture + the D-11 seam bake
3. **The gap record:** `176375239` (docs) — deferred-items + WINDOWS.md #43 (decision pending)

**Continuation segment (this executor):**
4. **RFMEM + the semaphore pair (r9):** `4298fc0` — rfork_copy's RFMEM arm + SHARED_PAGES + RFMEM families + the bss sweep + free/unwind pivots; SEMACQUIRE/SEMRELEASE; the xtask finish-scan Rule-1 fix; 6 host pins
5. **The resolution record (Harvey):** `f5bee06d6` — the deferred item + WINDOWS.md #43 marked fixed
6. **Task 2 RED (r9):** `9cc6482` — the five udp host pins (compile-error RED: the asserted udp API absent — the 08-02-sanctioned Rust RED shape)
7. **Task 2 GREEN (r9):** `9de69c7` — the udp module + conv spawn + dispatch arms + wake slots + listen drain + the read-side drive fix + xtask marker
8. **Task 2 Harvey half:** `391fb79f8` — the fixture's udp leg

**Plan metadata:** (Harvey repo — this SUMMARY + STATE/ROADMAP, committed after this write per the atomic close-out order)

## TDD Gate Compliance

- Task 2 was `tdd="true"`: RED commit `9cc6482` precedes GREEN `9de69c7`. The RED is a compile error (19 × error[E0425] naming exactly the planned-behavior surface — `udp_read_data`/`udp_rx_drain`/`udp_write_data`/`udp_advise` absent from proto), the Rust test-first RED shape when the asserted API does not exist (the 08-02 precedent); tdd.md's INVALID_RED categories do not apply.
- Two REAL red-glints inside the GREEN loop, both fixed in-loop and recorded: (1) the spawn pin needed `drive_stack()` after each scripted push (the drain reads the socket; scripted frames reach it only at poll); (2) two u64/usize type mismatches in the pins' own readbacks.
- No REFACTOR commit (nothing obvious to clean — the GREEN refactor extracted `udp_try_recv` as part of making the gate's wedge fix, recorded in the GREEN commit).
- `workflow.tdd_mode` is absent from config.json, so the runtime gate (`check tdd-red-evidence`) is inactive (the 08-02 note stands); the plan's commit-order text governed.

## Files Created/Modified

**r9:**
- `port/src/devip.rs` — the #I device surface (Task 1); udp joined the clone gate + QDATA read/write + QSTATS arms, UDP_RX/TX wake-slot arrays, listen_open's udp drain arm (Task 2)
- `port/src/devip/conv.rs` — the conv model (Task 1); `announcer`/`dataq`/`hangup` fields + `fs_new_call` (Task 2)
- `port/src/devip/medium.rs`, `ipifc.rs` — the medium registry + the loopback leg (Task 1)
- `port/src/devip/proto.rs` — the icmp module (Task 1); the udp module (Task 2)
- `port/src/devip_tests.rs` — the five Task-2 pins
- `port/src/dat.rs`, `port/src/namec.rs`, `aarch64/src/main.rs`, `aarch64/src/platform/virt/timer.rs` — the kernel wiring (Task 1)
- `aarch64/src/syscall.rs` — the DEV_IP pread/pwrite arms (Task 1); SEMACQUIRE/SEMRELEASE + sys_brk_'s family arm + the dispatch-table comment record (continuation + Task 2)
- `aarch64/src/proc.rs` — RFMEM: SHARED_PAGES, RFMEM_FAMILIES, rfork_copy's share arm + bss sweep, free_user_mappings/unwind pivots, the sem registry, rfmem_brk_set, the 6 host pins (continuation)
- `aarch64/lib/config_virtip.toml` — the dev=[..."ip"] profile (Task 1)
- `xtask/src/main.rs` — the virtip profile arm (Task 1); the finish-scan Netip exemption fix + the udp-echo marker (continuation + Task 2)

**Harvey:**
- `build/aarch64/root-tree/lib/netip-loopback.rc` — the gate fixture: the ping leg (Task 1) + the udp echo leg (Task 2)
- `.planning/phases/08-*/deferred-items.md` + `.planning/WINDOWS.md` — the RFMEM gap record, then its resolution

## Decisions Made

- **RFMEM shape (the checkpoint's decision A, implemented):** donor dupseg translated to r9's per-proc mapping_log model — Data/Bss pages share at fork (same PA in both procs' root tables, refcounted in SHARED_PAGES with last-sharer frees); Stack/Tos/Text stay fresh copies. The checkpoint's premise that "ping does not brk_-grow post-fork" was FALSIFIED at the gate (the receiver faulted unresolvable on the sender's malloc-grown page — the shared `first` list pointed into a parent-only-grown page), so the donor's shared-segment GROWTH semantics landed too: RFMEM families (rfork joins/founds; exec/exit dissolve in free_user_mappings) with sys_brk_'s family arm birthing grown pages shared into every member, plus the rfork bss sweep materializing untouched in-range pages shared at fork. Recorded in the phase deferred-items.md + WINDOWS.md #43 (both now `fixed`).
- **SEMACQUIRE(37)/SEMRELEASE(38):** 9k sysproc.c semantics — the counting word lives in user memory; wait queues keyed by the word's PHYSICAL address (RFMEM sharers contend on ONE queue); the register→check→block ordering is wakeup-loss-free by construction (single CPU, IRQ-masked syscall context). Era limit documented: the donor's note-interruptible sleep is not reproduced (r9's block is uninterruptible; libc's retry loop never fires). TSEMACQUIRE(52) stays loud-None. The syscall table grows by 2 — both NON-networking (the NETIP-02 "no networking syscalls / the file tree is the whole contract" prohibition is intact: no net syscall exists; libc dial/announce/listen unmodified).
- **No-cs service discovery:** the plan's text said udpecho announces `udp!*!7`; the baked binary announces `udp!*!echo`, and the research's "libc/nettrans service table" claim is falsified by announce.c/nettrans.c (identtrans passes the name through verbatim without cs — the DONOR's setladdrport likewise atoi's a name to a random port). The gate discovers the kernel-picked port from `/net/udp/N/local` — a nonzero port proves the announce ctl write completed (single-CPU syscall atomicity), so the socket is bound.
- **The conv-lifetime choreography:** a devip conv resets on its last close (closeconv), so rc's per-command `echo verb > ctl` shape loses the conv between commands (the udp leg's first form wedged on exactly this). The fixture holds the clone-opened ctl on fd 4 across the exchange via rc's `<>[4]` redirect — the rc(1) man page's own `newconn <>/net/tcp/clone` precedent.
- **The read-side drive:** a devip read path drives the stack before blocking (probe → drive → re-probe → register/arm → block). A write's egress only LOOPS the packet into the device; the pre-block drive's ingress moves it into the destination socket and fires its waker. The udp module's first form missed this (the icmp module carries it) — the loopback exchange deadlocked with both ends blocked until it landed.

## Deviations from Plan

### The checkpoint continuation (sanctioned work, recorded for the audit trail)

The tracer's Task 1 verify exposed the RFMEM gap (recorded pre-checkpoint in deferred-items.md + WINDOWS.md #43); the orchestrator returned a blocking-human decision checkpoint; the user selected option A (implement RFMEM now, Harvey-native requirement verified: Harvey's `sys/src/cmd/ip/ping.c:572` does `rfork(RFPROC|RFMEM|RFFDG)` and Harvey's 9k honors RFMEM in sysrfork). The continuation implemented it per the donor's dupseg semantics — and, when the gate falsified the checkpoint's growth premise, extended it to the donor's shared-segment growth (RFMEM families + the brk_ family arm + the bss sweep) plus the semaphore pair. ~700 lines in r9's proc.rs/syscall.rs (the checkpoint estimated ~150-200 for the fork-time-share subset alone). This resolves the gap recorded open in deferred-items.md and WINDOWS.md #43 (both marked fixed).

### Auto-fixed Issues

**1. [Rule 1 - Bug] xtask's finish()-time negative scan didn't exempt the fauth fall-through for virtip**
- **Found during:** continuation (the first post-RFMEM virtip run)
- **Issue:** Task 1's landing extended the mid-sweep scan to `netip_profile` but not the FINAL sweep in `finish()` (it passed only `profile == BootProfile::Net`) — so the real-chain boot's designed `syscall 10 unimplemented` (fauth, D-02) failed the virtip run at the post-check
- **Fix:** `matches!(profile, BootProfile::Net | BootProfile::Netip)` in `finish()`
- **Files modified:** ../r9/xtask/src/main.rs
- **Verification:** the virtip gate then ran to its real assertions
- **Committed in:** `4298fc0` (part of the RFMEM commit)

**2. [Rule 1 - Bug] udp read path missing the D-05 leg-3 read-side drive (loopback deadlock)**
- **Found during:** Task 2 GREEN (the udp echo gate wedged: dd and udpecho both blocked)
- **Issue:** my udp_read_data probed then blocked without driving; on loopback a write's egress only loops the packet into the DEVICE — the pre-block drive's ingress moves it into the destination socket and fires its waker (icmp_read_data carries this shape; I missed it)
- **Fix:** extracted `udp_try_recv` and restructured to probe → drive → re-probe → register/arm → block
- **Files modified:** ../r9/port/src/devip/proto.rs
- **Verification:** the udp echo gate passed; all 66 host pins green
- **Committed in:** `9de69c7` (GREEN)

**3. [Rule 1 - Plan text bug] udpecho announces `udp!*!echo`, not `udp!*!7` — and no libc service table exists**
- **Found during:** Task 2 (the gate's first form: the fixture's poll for port 7 never matched — the server announced a kernel-picked random port)
- **Issue:** the plan's action text parenthesized "(the `echo` service name maps to port 7 via the libc/nettrans service table)" — announce.c's identtrans passes the name through VERBATIM without cs (and the donor's setladdrport atoi's a name to a random port); udpecho is byte-pristine (baked 08-03, unmodified per the acceptance criteria)
- **Fix:** the fixture discovers the server's real port from `/net/udp/N/local` (a nonzero port proves the announce completed) and connects to it — the honest no-cs gate
- **Files modified:** build/aarch64/root-tree/lib/netip-loopback.rc
- **Verification:** the udp echo gate passed with udpecho unmodified
- **Committed in:** `391fb79f8`

**4. [Rule 1 - Bug] the udp leg's first fixture form died on closeconv (conv-lifetime)**
- **Found during:** Task 2 (the wedge after the port discovery: rc's per-command `echo verb > ctl` opens+closes the conv's files, so the conv reset between commands — the write went to a fresh Idle conv with no socket and the read wedged)
- **Fix:** the clone-opened ctl is held on fd 4 across the whole exchange (rc's `<>[4]` redirect — the man page's newconn precedent)
- **Files modified:** build/aarch64/root-tree/lib/netip-loopback.rc
- **Verification:** the udp echo gate passed
- **Committed in:** `391fb79f8` (same commit)

**5. [Rule 2 - Missing Critical] SEMACQUIRE(37)/SEMRELEASE(38) — the plan's "syscall table unchanged" verification note yields to the RFMEM decision**
- **Found during:** continuation (the first post-RFMEM ping run: 19+ `syscall 37 unimplemented` lines — a dead child's stuck lock spun the parent's `while(semacquire(...)<0)` retry forever)
- **Issue:** libc's `lock()` contended path (`while(semacquire(&l->sem, 1) < 0)`) is reachable only WITH shared memory — r9 never implemented the pair because RFMEM was never honored. The plan's verification prose says "the syscall table unchanged"; its NETIP-02 prohibition is specifically about NETWORKING syscalls / non-file side channels (the /net tree is the whole networking contract — intact). The pair are the donor's standard process-synchronization syscalls, and the user-sanctioned RFMEM makes them required; there is no alternative path (ping is byte-pristine)
- **Fix:** implemented with 9k sysproc.c semantics (PA-keyed wait queues; wakeup-loss-free ordering by construction; the uninterruptible-block era limit documented); TSEMACQUIRE(52) stays loud-None; the dispatch-table comment records the growth and why
- **Files modified:** ../r9/aarch64/src/syscall.rs, ../r9/aarch64/src/proc.rs
- **Verification:** the ping gate passed; 8/8 regression profiles green
- **Committed in:** `4298fc0`

**6. [Rule 1 - My own code] two warning classes in the udp module + pin type fixes**
- unused `lport` destructure in udp_write_data (the bind pins the source port — the man's lport-override rule is by-construction; comment recorded), `do_wake` never-read in udp_advise (simplified), `udp_advise` dead-code in the non-test build (§12.7's recorded unwired corner; `#[allow(dead_code)]` with the note), and the pins' u64/usize + tuple-destructure fixes
- **Committed in:** `9de69c7` (GREEN)

---

**Total deviations:** 6 (5 auto-fixed + the sanctioned checkpoint continuation). 
**Impact on plan:** Every deviation either fixes my own/Task-1's code or corrects plan/research text against the SOURCE (the contract doc's own authority rule). The syscall-table growth is the one that bends plan verification prose — it is the user-sanctioned RFMEM decision's required companion, non-networking, and recorded in-tree (dispatch comment) + in STATE.md decisions. No locked decision changed; NETIP-02's contract intact.

## Issues Encountered

- **`git stash` hazard (self-inflicted, recovered):** while checking whether the unused-import warnings were pre-existing, a `git stash -q --include-untracked -- aarch64/src/syscall.rs` swept up my syscall.rs edits (the path filter held — exec.rs was never swept). Recovered intact via `git stash show --name-only` verification + `git stash pop` of exactly `stash@{0}` (the pre-existing `stash@{1}` left untouched). No data lost. The pre-existing r9 `aarch64/src/exec.rs` modification was never touched, staged, or committed anywhere in this plan.
- **gsd_run/gsd-tools shim unavailable** in this environment (the phase's standing condition) — STATE.md/ROADMAP.md hand-edited to the same fields the verbs maintain (position, metric row, decisions, progress count; roadmap plan flip to 4/9). `requirements.mark-complete` was correctly a no-op anyway: NETIP-02/NETIP-03 are shared with pending plans (shared-ID gate #2388). Note: STATE.md's completed_plans read 23 at my edit time (the concurrent phase-06 session had moved it past the 08-03 executor's 21 with its own plans) — my bump to 24 reflects this plan; the global phase pointer moved 06→08 because phase 06's last plan (06-10) carries a SUMMARY and its session ended at "verifying".
- **Concurrent-session traffic:** 24 phase-06 commits landed in the plan's Harvey ledger window (docs(06-*), test(06), the review records) — disjoint files, no conflicts; recorded here so the measured commit count reconciles.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **08-05 (tcp proto) unblocked:** the conv model + AcceptPool (08-02) + the read-side drive discipline + the conv-lifetime choreography are all landed and proven; the udp module is the shape tcp follows; netlog/stats/iproute/ipselftab/ndb surface lands there per its plan
- **08-06/08-07 notes:** the udp module's v6 corners are recorded in-code (a v6 destination on the v4 udp conv errors "bad ip address syntax"; v6 senders in the drain are skipped) — 08-06's udp6/icmpv6 legs pick them up; the DHCP gate rides the udp module's broadcast path (the OQ-1 bridge rule governs: local_address from the conv's laddr)
- **RFMEM consumers beyond ping:** libthread's procs (rfork with RFMEM) now work — the note about one-member-family lazy shape and the demand-page-sharing refinement (text/stack faults post-fork stay per-proc) are recorded in proc.rs
- r9's pre-existing uncommitted `aarch64/src/exec.rs` modification was left untouched and out of every commit, as instructed
- No blockers

## Self-Check: PASSED

- r9 commits `fdff261`, `4298fc0`, `9cc6482`, `9de69c7` exist on r9 main (`git log --oneline 63a3ab7..HEAD`) — FOUND
- Harvey commits `fd8a9034c`, `176375239`, `f5bee06d6`, `391fb79f8` exist on master — FOUND
- `target/boot-test-virtip.log` contains `#I0: devip attached`, `devip: medium loopback bound`, `netip: ping ok`, `netip: udp echo ok` (after ping ok), `netip: NETIP-DONE`; zero negative-scan hits — VERIFIED
- `cargo test -p port devip -- --nocapture` → 12 passed, 0 failed; full port suite 66/66; `cargo test --package aarch64 --bins` 14/14 (6 rfmem pins) — VERIFIED at execution time
- `cargo xtask check --verbose` + `cargo xtask clippy --verbose` exit 0, no new warnings naming this plan's files — VERIFIED
- All 8 regression profiles (virt virtlowecam virtexec virtexec3 virtmulti virtnotes virtpanic virtnet) PASS — VERIFIED at execution time
- `../r9/port/src/dat.rs` contains `DEV_IP: usize = 9`; namec.rs the 'I' arm + DEV_IP dispatch arms; syscall.rs the DEV_IP arms with drain_debt_if_set first (2 sites); main.rs the cfg(dev_ip) hook registration; timer.rs the cfg(dev_ip) sweep — VERIFIED (grep)
- `../r9/port/src/devip.rs` carries `#![forbid(unsafe_code)]` — VERIFIED
- `bind null` produces exactly `cannot bind null device` (medium.rs, 2 sites) — VERIFIED (grep)
- `../r9/aarch64/lib/config_virtip.toml` exists with `dev = ["exec", "bootchain", "ether", "ip"]` — VERIFIED
- `build/aarch64/root-tree/lib/netip-loopback.rc` exists, sources /lib/netaddr.rc, carries the `netip: FAIL <stage>` / `netip: NETIP-DONE` vocabulary — VERIFIED
- The NETIP-02 transparency prohibition: libc + ping.c + udpecho.c byte-untouched across the plan window (`git diff --stat 499b4f00e..HEAD -- sys/src/libc sys/src/cmd/ip/ping.c` empty; udpecho's edits date to 08-03's payload work, not this plan) — VERIFIED
- raspi3b/raspi4b/x86_64/riscv64 stay green by construction (xtask check compiles all arch bins; the devip wiring is cfg(dev_ip)-gated) — VERIFIED
- r9 `aarch64/src/exec.rs` pre-existing modification untouched and uncommitted — VERIFIED (git status)

---
*Phase: 08-r9-tcp-ip-path-devip-compatible-net*
*Completed: 2026-11-05*
