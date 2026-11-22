---
phase: 18-in-guest-p9sk1-auth-domain
plan: "07"
subsystem: kernel (r9 devip TCP delivery — the AUTH-08 kernel half)
tags: [r9, devip, tcp, smoltcp, nagle, delayed-ack, secstore, two-write, sconn, auth-08, loopback]

requires:
  - phase: 18-05 (the secstore stall record + isolation matrix)
    provides: the stall-mode evidence this plan's reproduction re-ran (mode-1 control / mode-2 stall; the alarm-note symptom)
  - phase: 18-06 (pushed r9 base f594b4c)
    provides: the clean kernel base the fix lands on (batteries 146/146 + 86/86)
provides:
  - "devip two-write delivery: SConn's donor client pattern (write(count,2) then write(payload,n)) round-trips over r9's loopback TCP — the donor-faithful socket config (Nagle OFF, no delayed ACK) at proto.rs::tcp_socket, pinned RED->GREEN host-side and proven live in-guest"
  - "the RED two_write host pin: tcp_two_write_client_pattern_round_trips (the REAL loopback core, the REAL tcp_write_data/tcp_read_data paths, count-then-payload ORDER asserted)"
  - "live mode-2 round-trip re-proof: the before/after capture pair under evidence/ (the same probe, unfixed vs fixed working-tree kernel)"
affects: [18-08 (the FOD pass carries ab4cb66 into the pin; then the sweep 9d restore + the gate's absent-assert flip close AUTH-08's Harvey half), Phase 19 cpu path (multi-write TCP rides the seam)]

actuals:
  tokens: 12300   # chars/4 over realized changes: r9 diff 6731c + evidence 32118c + this SUMMARY ~10000c (estimate was 40000 — the authored r9 content is ~1.7k tokens; the evidence retention is the plan's own design)
  tasks: 3
  commits: 1      # r9 production commits: ab4cb66 (the ONE class commit per the plan's own instruction); harvey docs commit follows (the plan ledger convention — Task 1/3 are capture-only on this side)
plan_head_before: 31b9b5c2dcee992d731687fb65dd8dac7fe51c62
plan_head_after: 31b9b5c2dcee992d731687fb65dd8dac7fe51c62   # harvey HEAD unchanged by production (r9 carries the bytes); this SUMMARY's docs commit sits directly atop

tech-stack:
  added: []
  patterns:
    - "name-grepped RED pin over the REAL loopback core: the park itself is converted into the pin's failure by the harness's ON_BLOCK one-shot (the threaded shape races the seam's BusyCell — the harness's own rejected-threading discipline), so the guest stall is exhibited deterministically host-side"
    - "the conv status file (/net/tcp/N/status qin/qout) as the in-guest hold instrument: qout = the socket's send-queue bytes (unsent + sent-unacked) made the Nagle hold visible live (2 -> 33 across the payload write, still 33 at t+3s)"

key-files:
  created:
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-07-devip-twowrite-repro.console.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-07-devip-twowrite-fix.console.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-07-t2-red-evidence.json
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-07-t2-red-raw-libtest.json
  modified:
    - ../r9/port/src/devip/proto.rs        # tcp_socket: set_nagle_enabled(false) + set_ack_delay(None) — the donor's TCP shape, with the full mechanism comment
    - ../r9/port/src/devip_tests.rs        # tcp_two_write_client_pattern_round_trips (the RED->GREEN pin)

key-decisions:
  - "The defect site is tcp_socket's smoltcp stock defaults, not the drive/wake machinery: Nagle ON + delayed ACK 10ms hold every sub-MSS follow-up write until a post-deadline drive that a fully-parked venue can never run — the fix restores the DONOR's TCP shape (Plan 9 sends immediately and ACKs immediately; its loopbackmedium is synchronous by design), platform-neutral in port/ so both arches get it and aarch64's own tree stays byte-identical"
  - "ack_delay(None) is load-bearing beyond Nagle: the 10ms delayed ACK also gated tx-buffer drain — a >2048-byte bulk write (secstore PUT, Phase 19 exportfs) would block in tcp_write_data's send arm on the same never-driven timer"
  - "The x86_64 tick_check gap is recorded, NOT fixed here: x86_64's q35 timer tick never calls stack::tick_check (the 08-04 deadline leg is aarch64-only wiring), so retransmit/keepalive/connect-timeout timers still never fire on x86_64 with all devip procs parked — adjacent latent defect, outside the diagnosed pattern and the plan's file set; reopener recorded in deferred-items"
  - "The reproduction instrument stays UNCOMMITTED (the 18-05 precedent): the hsecprobe C probe + the temporary mkimage/sweep legs were reverted after each capture; the retained captures are the evidence"

requirements-completed: [AUTH-08]   # plan frontmatter verbatim; the kernel half closed here — the REQUIREMENTS checkbox stays held by the shared-ID gate until 18-08 (the co-declarer owning the sweep 9d restore + the gate flip) lands its summary

coverage:
  - id: D1
    description: "The mechanism record: the defect named at file:function with trace evidence from three independent venues (source, host seam, live guest)"
    requirement: AUTH-08
    verification:
      - kind: other
        ref: "SUMMARY section THE MECHANISM RECORD below (smoltcp source cites + the host DIAG lines + the repro capture's qout evolution)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The RED->GREEN host pin at the diagnosed level: the two-write pattern over the REAL loopback core through the REAL tcp_write_data/tcp_read_data paths, count-then-payload order asserted (T-18-17)"
    requirement: AUTH-08
    verification:
      - kind: unit
        ref: "cargo test -p port --lib -- two_write => 1 passed; RED pre-fix RED_EVIDENCE_OK (evidence/18-07-t2-red-evidence.json: 14 discovered, 13 passed, only the target failed — on the planned park assertion); GREEN post-fix; full port battery 147/147 (146 baseline + the pin)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The fix at the diagnosed site: donor-faithful tcp sockets (Nagle off, no delayed ACK) — no busy-wait, no timeout unblock, the register-then-block discipline and NetWaiterScope pairing untouched, aarch64 byte-identical"
    requirement: AUTH-08
    verification:
      - kind: unit
        ref: "r9 ab4cb66: diff touches exactly proto.rs::tcp_socket + the pin (99 insertions, 0 deletions); the workaround grep (sleep/spin/timeout in the tcp paths) is empty; x86_64 bins 86/86; xtask check + clippy exit 0; git diff -- aarch64 empty"
        status: pass
    human_judgment: false
  - id: D4
    description: "The live working-tree re-proof: the mode-2 count+payload exchange DELIVERS at the fixed kernel (the peer reads the pair, the framed two-write reply reaches the parked client), mode-1 still round-trips, the gate ALL STAGES PASS with the SKIP arm intact (18-08 owns the flip)"
    requirement: AUTH-08
    verification:
      - kind: e2e
        ref: "evidence/18-07-devip-twowrite-fix.console.log:514-516 `mode-2 REPLY count read (2 bytes, len 35)` + `mode-2 REPLYTEXT !protocol should start with ver alg` + `mode-2 child done err=` (the reader child COMPLETED); m2-t1s/t3s `Close_wait qin 0 qout 0` (drained); the repro capture (unfixed) shows the same exchange parked with qout 33 at t+3s"
        status: pass
    human_judgment: false
  - id: D5
    description: "The regression surface: q35net boot-test green under the fix (bind, ping-pong, tx-bulk, TCP echo substrate); the one-write control unchanged; r9 main pushed"
    requirement: AUTH-08
    verification:
      - kind: e2e
        ref: "target/boot-test-q35net.log: `netgate: ping-pong done 256/256` + `netgate: tx-bulk done 4096` + X64-NET-DONE; PUSHED-AARCH64-CLEAN (origin/main == ab4cb66, unpushed empty, aarch64 diff empty)"
        status: pass
    human_judgment: false

duration: 34min
completed: 2026-11-14
status: complete
---

# Phase 18 Plan 07: devip Two-Write Delivery (AUTH-08 Kernel Half) Summary

**r9's devip now delivers SConn's donor two-write TCP pattern — the defect was tcp_socket's stock smoltcp defaults (Nagle + 10ms delayed ACK) holding every sub-MSS follow-up write for a post-deadline drive that a fully-parked venue can never run; the donor-faithful socket config (Nagle off, ACK immediate) is pinned RED->GREEN host-side over the real loopback core and proven live in-guest (the mode-2 stall became the mode-2 round-trip), batteries 147/147 + 86/86, q35net green, r9 pushed at ab4cb66.**

## Performance
- **Duration:** 34 min | **Started:** 2026-11-14T15:52:42Z | **Completed:** 2026-11-14T16:26:45Z | **Tasks:** 3/3 | **Files modified:** 6 (2 r9 source, 4 evidence/docs)

## THE MECHANISM RECORD (the defect named, with evidence)

**Defect site: `../r9/port/src/devip/proto.rs::tcp_socket`** — every devip tcp socket was created with smoltcp 0.14's stock defaults: **Nagle enabled** (`nagle: true`, smoltcp socket/tcp.rs:620) and **delayed ACK at 10 ms** (`ack_delay: Some(ACK_DELAY_DEFAULT)`, socket/tcp.rs:617). On the loopback medium the MSS is ~16.3 KB (LOOPBACK_MTU 16384), so NO SConn-sized write is ever a full segment.

The causal chain, each step measured:

1. `tcp_write_data` (proto.rs:2564) accepts the second write into the socket's tx buffer and returns success — the client's write syscall never blocks — but smoltcp's egress dispatch holds the segment: `if self.nagle && data_in_flight && !can_send_full && !want_fin { can_send = false; }` (socket/tcp.rs:2362): the previous small segment is unacked and the follow-up is far below MSS. **Live guest evidence:** qout 2 → 33 across the payload write, and STILL 33 at t+1s and t+3s (evidence/18-07-devip-twowrite-repro.console.log:504-514).
2. The only release is the peer's pure ACK, which the peer's socket delays 10 ms (`AckDelayTimer::Waiting(now+10ms)` armed at data ingress, socket/tcp.rs:2255-2265); the ACK is emitted only by an egress at/after the deadline, and `drive_stack` arms `NET_DEADLINE` from `poll_at` (stack.rs:607-615). **Host seam evidence:** `DIAG mode2: ... deadline=Some(10)` with `client_tx` accumulating and the peer's rx empty.
3. When the deadline passes, the only wake path is `stack::tick_check` (stack.rs:340) — wired on aarch64 (platform/virt/timer.rs, 08-04) but NEVER wired into x86_64's q35 timer tick (q35/timer.rs::tick runs timeouts/preempt/console only). Every `drive_stack` caller is a devip file op (proto.rs/conv.rs); with both procs devip-parked (the client reading the reply in `tcp_read_data`'s block arm, the server in `readn(31)`), no devip op ever runs again → the delayed ACK never egresses → Nagle never releases → **both procs park forever**.
4. **Host-side determinism:** over the REAL loopback core, write(2)+write(31) left `peer_rx=0`/`peer_rx=2` with the peer's real `tcp_read_data` parking forever — the same stall, reproduced in the pin harness (the temporary diagnostic; the permanent pin is Task 2's).

**Why mode-1 and the server-direction two-write survive the same flaw:** a direction's FIRST small segment has nothing in flight — Nagle allows it out in the write's own terminal drive (host-measured: a single 33-byte write delivered instantly, peer_rx=33). The server's reply IS a two-write pattern, and its 35-byte tail IS initially held — but its first 2-byte segment DELIVERS to the parked client, whose recv waker fires (socket/tcp.rs:2246): the client is READIED, so it RUNS, and its pre-block drive (proto.rs:2511) is a post-deadline drive candidate — in the busy auth venue (the churn loop interleaves) 10+ guest ticks elapse before that drive, the delayed ACK has expired, and the SAME drive releases Nagle and lands the tail (observed: REPLY 37 bytes in both the 18-05 and this plan's captures). In mode-2 NOBODY is readied — the held payload delivers nothing, so no post-deadline drive can ever exist. **The mode-2 stall is structural; mode-1's delivery is venue-timing luck riding the same flaw.**

**Lowest exhibitable pin level:** the seam itself, host-side — the real loopback core + the real `tcp_write_data`/`tcp_read_data`: after write(count,2)+write(payload,31) the peer must return BOTH writes in order with no clock advance. Unfixed, the payload read parks (the harness's ON_BLOCK one-shot converts the park into the pin's RED failure — the threaded shape races the seam's BusyCell, the harness's own rejected-threading discipline).

## Accomplishments
- **AUTH-08's kernel half closed:** SConn's donor two-write client pattern round-trips over r9's loopback TCP — pinned deterministically (`tcp_two_write_client_pattern_round_trips`, RED_EVIDENCE_OK pre-fix) and proven live (the before/after capture pair: the same probe, the unfixed kernel stalls with qout 33 at t+3s; the fixed kernel reads the full framed error reply back through the parked client's reader child)
- **The mechanism record written** (above) — the defect named at file:function across three venues (smoltcp source, the host seam, the live guest), with the mode-1/server-direction survival explained (first-segment egress + the readied client's post-deadline drive)
- **The fix is donor-faithful and minimal:** two socket-config calls at the diagnosed site; no busy-wait, no timeout unblock, the register-then-block discipline and the NetWaiterScope pairing untouched, platform-neutral in port/ (aarch64 byte-identical)
- **The full regression surface green:** port battery 147/147 (146 baseline + the pin), x86_64 bins 86/86, xtask check + clippy clean, q35net boot-test green (ping-pong 256/256, tx-bulk 4096, X64-NET-DONE), the one-write control (mode-1) unchanged in both captures, the auth gate ALL STAGES PASS twice (repro + fix runs) with the SKIP arm intact
- **r9 main pushed at ab4cb66** (push-before-pin; 18-08's single FOD pass covers 18-06+18-07 bytes together) — NO FOD re-wire, NO harvey flake.lock change, NO gate-SKIP change this plan (all verified: the r9 commit touches only proto.rs + devip_tests.rs; the harvey tree carries no flake.lock diff)

## Task Commits
1. **Task 1: reproduce at the working tree + the mechanism record** — no commit (pure diagnosis by design; the capture + the record are the deliverables; both working trees verified clean of diagnostics after)
2. **Task 2: RED pin + the fix + batteries green** — r9 `ab4cb66` (fix; the ONE class commit carrying the pin and the fix together, per the plan's own 17-04-precedent instruction — see TDD Gate Compliance)
3. **Task 3: live re-proof + q35net + push** — no new commits (the deliverables are the fix capture, the green q35net, and the push; r9 origin/main == ab4cb66)

**Plan metadata:** this SUMMARY + STATE/ROADMAP/REQUIREMENTS (the docs commit follows)

## Files Created/Modified
- `../r9/port/src/devip/proto.rs` — tcp_socket: `set_nagle_enabled(false)` + `set_ack_delay(None)` (the donor's TCP shape) + the full mechanism comment
- `../r9/port/src/devip_tests.rs` — `tcp_two_write_client_pattern_round_trips` (the loopback-core RED->GREEN pin, ON_BLOCK park bound, order asserted)
- `evidence/18-07-devip-twowrite-repro.console.log` — the UNFIXED working-tree kernel: mode-1 round-trips, mode-2 stalls (qout 2 → 33, still 33 at t+3s, reader parked) — 547 lines, gate ALL STAGES PASS
- `evidence/18-07-devip-twowrite-fix.console.log` — the FIXED working-tree kernel: mode-1 unchanged, mode-2 DELIVERS (REPLY count read + REPLYTEXT + child done; Close_wait qout 0) — gate ALL STAGES PASS
- `evidence/18-07-t2-red-evidence.json` + `18-07-t2-red-raw-libtest.json` — the machine-classified RED record (RED_EVIDENCE_OK) + the unmodified libtest stream

## THE BEFORE/AFTER PAIR (side by side)

| Observable (same probe, live secstored listener) | UNFIXED (repro capture) | FIXED (fix capture) |
|---|---|---|
| mode-1 single 33-byte write | REPLY 37 bytes (`!protocol should start with ver alg`) | identical |
| mode-2 after write(count,2) | qout 2 (the count segment out, unacked) | qout 2 (same shape) |
| mode-2 after write(payload,31) | qout 33 — the 31 bytes HELD | qout 33 at the instant, then drained |
| mode-2 at t+1s / t+3s | qout STILL 33, Established (the hold never releases) | `Close_wait qin 0 qout 0` (the exchange completed and drained) |
| the parked reader child | parked forever (`reader still parked at t+3s`) | `REPLY count read (2 bytes, len 35)` + `REPLYTEXT !protocol should start with ver alg` + `child done err=` |
| the gate | ALL STAGES PASS (the SKIP arm holds) | ALL STAGES PASS (the SKIP arm holds — 18-08 flips it) |

(Note: the fix capture's line `mode-2 reader still parked at t+3s (the stall) pid=79` is the instrument's UNCONDITIONAL parent print — the capture's own ordering disproves it: `mode-2 child done` (line 516) precedes `m2-t1s` (line 521). The parent prints the canned stall line regardless of the child's state; only the child's completion lines and the qout/Close_wait evolution are load-bearing.)

## Decisions Made
- See key-decisions: the tcp_socket defect-site framing (not the drive/wake machinery); ack_delay(None) as load-bearing for bulk writes; the tick_check gap recorded-not-fixed (adjacent latent defect, reopener in deferred-items); the uncommitted-instrument precedent

## TDD Gate Compliance
Task 2 (tdd="true") followed the plan's own ONE-class-commit instruction (the 17-04 Pitfall-5 convention, restated verbatim in the plan's Task-2 action): **RED evidence was captured and machine-classified BEFORE any implementation** (`gsd_run check tdd-red-evidence` over evidence/18-07-t2-red-evidence.json → RED_EVIDENCE_OK, reason target_test_failed: 14 tests discovered, 13 passed, only `devip_tests::tcp_two_write_client_pattern_round_trips` failed — on the planned park assertion, with the 13 tcp/iproute siblings green through the same real paths); the pin then rode the ONE class commit `fix(18-07):` (r9 ab4cb66) together with the fix — **no separate test() commit exists for 18-07, exactly as the plan directs**. GREEN verified post-fix (pin 1/1, full battery 147/147). Task 1 is a tracer (not tdd): executed, verified, committed nothing (by design), and the tracer feedback gate re-ran both verifies green before expansion.

## Deviations from Plan

None - plan executed exactly as written.

### Verify adaptations (not deviations)
- **Task 1's HARVEY-TRACKED-CLEAN check excludes `sys/src/cmd/eqn/eqn.c`** — the pre-existing local scratch modification the dispatch explicitly forbids touching or committing ("stays out of every commit"). Every file this plan touched verified clean; the exclusion is documented rather than silently widening the check. (The repro-retention and r9-clean checks ran unmodified and passed.)
- The reproduction/fix gates ran with `R9_SKIP_BUILD=1` (the driver's documented iteration hatch) and `--override-input r9 git+file:///home/req/code/r9` (the flake's own documented dev seam) — the working-tree kernel without touching the harvey flake.lock (which stays at r9 b2a6151, as the dispatch requires).

**Total deviations:** 0. **Impact:** none — the plan's file set and semantics landed exactly as specified.

## Issues Encountered
- The first host diagnostic run compiled+spun (the peer read parks forever in the harness's stub-block loop when the payload is held — the stall itself, host-side); the run was killed, the spin documented as mechanism evidence, and the temporary test reverted (the permanent pin replaces it with the ON_BLOCK park bound)
- The probe's parent prints an unconditional "reader still parked" line (see the before/after note) — cosmetic only; the child's completion lines are the delivery proof
- The mode-2 reply's REPLYTEXT shows two trailing garbage bytes (`...ver alglg`) — the reply's own 35-byte frame includes the digest-slot padding PAKserver sends; readn delivered exactly len bytes; not a defect

## Authentication Gates
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- **18-08 can run its single FOD pass over 18-06+18-07 bytes together** (r9 main == ab4cb66, pushed; the two-derivation proof then re-wires the boot-iso FOD hash and advances the harvey flake.lock), then restores the sweep's 9d legs and flips the gate's absent-assert to presence demands — AUTH-08's Harvey half (the payload object, the account machinery, and the venue are all in place and green from 18-05; the kernel now delivers the pattern)
- **Phase 19's cpu path inherits a sound seam:** multi-write TCP (exportfs over cpu connections) no longer depends on timer-mediated release for delivery; the bulk-write (>2048-byte) send-arm stall class is closed by the same ack_delay(None)
- **Deferred (recorded):** x86_64's unwired `tick_check` deadline leg — retransmit/keepalive/connect-timeout timers still never fire on x86_64 when all devip procs park (aarch64 has the leg since 08-04); no current gate exercises it; the reopener is the first venue that needs timer-mediated TCP progress under full park (an ether-loss retransmit venue, or a connect-timeout gate)

## Self-Check: PASSED

All key files exist on disk (proto.rs + devip_tests.rs at r9 ab4cb66; all four evidence files in the phase evidence dir; this SUMMARY); the r9 commit verified as HEAD and == origin/main (unpushed empty); aarch64 diff empty; the harvey tree carries no diff beyond this docs set (eqn.c pre-existing scratch untouched); both captures carry the full secprobe marker set with the gate's ALL-STAGES-PASS verdict; flake.lock untouched in both repos; no FOD or gate-SKIP byte changed by this plan's commits.

---
*Phase: 18-in-guest-p9sk1-auth-domain*
*Completed: 2026-11-14*
