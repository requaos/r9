---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "08"
subsystem: kernel-transport-crypto
tags: [9p2000-aead, rekey, in-band-exchange, marker, ack-confirm, sequence-counter, zeroize, fail-closed, sans-io-clock]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-07)
    provides: "port::chan::transport::{Session, Handshake, fail()}, the sealed engine, keys_zeroized, the in-memory AeadLink fixture and the exchange seam"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-16)
    provides: "the strict per-direction sequence check, SendHalf and its send lock, the replay/reflect knobs"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-05)
    provides: "msg::{Trekey, Rrekey, REKEY_REQUEST, REKEY_MARKER, REKEY_ACK}, encode_body/decode_body and the P5 data layout"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-03)
    provides: "crypto::session::{derive, Transcript::chain, DirectionKeys::{rekey_confirm, verify_rekey_confirm}}"
provides:
  - "Session::{rekey_due, rekey_step, poll, superseded_keys}; transport::Superseded and transport::SEQ_HARD_LIMIT"
  - "the in-band re-key: a fresh Tauth exchange sealed inside the channel through SessionExchange, chained into the transcript"
  - "per-direction, strict cutover: the marker is the last old-key packet; send keys switch and the counter returns to 0 under the new keys"
  - "the acknowledgement with a confirm value under the new receive-direction MAC key; the sender zeroizes its old keys on a valid ACK"
  - "a caller-driven deadline (poll/seal/open/rekey_step) with no clock read; a wrong confirm, a wrong generation or a missed ACK closes"
  - "the hard counter limit: sequence u64::MAX is never sent; Handshake::finish_with_send_seq for the host limit test only"
  - "the 13 rekey pins, including the independent wire pin aead_transport_rekey_wire_keys_and_confirm_equal_test_side_derivation"
affects: [04-13 (RFC sections 3.2 and 11 cite the independent pin), 04-17 test-9p, phase 05 page reads, phase 06 (a live mount must call poll(now))]

# Actuals (#2632): the same estimateTokens scale as the plan's `estimate`.
actuals:
  tokens: 22748
  tasks: 2
  commits: 4
  plan_head_before: a559dc0be8cad32b7a7ad52653e356c8e866a690
  plan_head_after: d409f4be1f4790c1564c051006b3966215c0468c

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "re-key state: pending keys indexed by direction, a chained Transcript, an exchange phase, a ready flag and one superseded slot per direction"
    - "cutover atomicity: SendHalf::marker_and_switch seals the marker at the current seq, moves the old keys into the superseded slot (Held), installs the new keys, resets the counter to 0 and stores the deadline, all under the one send lock (D-21)"
    - "superseded_keys reads is_zeroized() on the slot, never a flag: receiver Zeroized at the marker, sender Held until the ACK then Zeroized"
    - "the exchange transcript chains from the previous generation's hash and absorbs each in-band Tauth/Rauth/Twrite/Rwrite/Tread/Rread in its standard 9P2000 form, whole, in wire order"
    - "sans-IO time: the deadline is now + rekey_ack_timeout_ms, checked by poll, seal, open and rekey_step; the transport names no clock"
    - "the receiver's ACK confirm is rekey_confirm(g+1) under the new receive-direction MAC key, so sealing under the other direction proves nothing"
    - "the independent pin re-derives generation 1 from the recorded plain handshake frames and the sealed exchange frames, decrypting bodies with literals (4096-byte pages, 64 blocks, params 16/4) and never calling the transport's opener"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-08.md
  modified:
    - port/src/chan/transport.rs
    - port/tests/common/aead_link.rs
    - port/tests/aead_transport.rs

key-decisions:
  - "Both ends derive generation g + 1 for both directions from one exchange; each end then cuts over only its own send direction on its own marker, so the two directions cut over independently"
  - "The ACK for a direction travels in the opposite direction and its confirm is made under the new keys of the direction cut over, not of the direction that carries the ACK"
  - "The sender's old keys live in SendState (Held, then zeroized on the ACK); the receiver's old keys live in a Session::rekey slot (zeroized at the marker) — one superseded slot per direction per end"
  - "The exchange is always client-driven; a server that reaches the threshold first sends Rrekey kind REQUEST and the client starts Tauth"
  - "The hidden Handshake::finish_with_send_seq is a #[doc(hidden)] host-only constructor; TransportConfig gains no start-counter field and Handshake::finish always starts at 0 (D-17)"
  - "A re-key transcript absorbs only the six in-band exchange messages in standard form; sealed bytes, headers, TLVs, padding and the MAC are not absorbed"
  - "Task 1 disables the acknowledgement entirely; Task 2 RED adds a zero-confirm ACK stub, then Task 2 GREEN makes the confirm real"

patterns-established:
  - "a re-key exchange is driven by a caller that hands decoded messages into rekey_step and puts the returned sealed packets on the wire; the transport seals them itself"
  - "the test-side wire pin opens sealed bodies with its own DirectionKeys and P5 literals, builds the standard frames by hand and compares MACs and confirm values against the transport's bytes"
  - "withhold_rekey_ack drops the server's Rrekey ACK at the fixture, so a deadline can be exercised without the peer answering"

requirements-completed: [WIRE-04]

coverage:
  - id: D1
    description: "A re-key is due when the send counter reaches the configured threshold; the in-band exchange travels sealed, and a server that reaches the threshold first asks the client to start it"
    requirement: WIRE-04
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_fires_at_threshold"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_server_requests"
        status: pass
    human_judgment: false
  - id: D2
    description: "Per-direction strict cutover: the marker is the last old-key packet, the next packet is sequence 0 under the new keys, the receiver zeroizes its old keys at the marker and the sender holds them until the ACK; the directions cut over independently and a same-secret seam still changes the keys"
    requirement: WIRE-04
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_marker_then_new_keys_at_zero"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_directions_cut_over_independently"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_same_secret_still_changes_keys"
        status: pass
    human_judgment: false
  - id: D3
    description: "The acknowledgement carries the new generation and a confirm under the new receive-direction MAC key; the sender verifies it and zeroizes its old keys, and a wrong confirm or wrong generation closes the connection"
    requirement: WIRE-04
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_marker_then_ack"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_wrong_confirm_closes"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_ack_for_wrong_generation_closes"
        status: pass
    human_judgment: false
  - id: D4
    description: "The caller-driven deadline closes the waiting sender with no clock read; an old-key packet is rejected after the marker; the send counter never wraps and u64::MAX is never sent"
    requirement: WIRE-04
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_ack_timeout_closes"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_no_old_key_packet_after_marker"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_counter_never_wraps"
        status: pass
    human_judgment: false
  - id: D5
    description: "The re-key inputs are independent of the transport: the test derives generation 1 from the recorded frames and finds the first post-marker packet's MAC and each ACK's confirm match its own derivation and not the other direction, generation 0 or generation 2"
    requirement: WIRE-04
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_wire_keys_and_confirm_equal_test_side_derivation"
        status: pass
    human_judgment: false

# Metrics
duration: 8min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 8: In-band re-keying with strict per-direction cutover Summary

**WIRE-04 complete on the host: a threshold-triggered in-band Tauth exchange derives generation g + 1, each direction cuts over on its own marker to sequence 0 under new keys, an acknowledgement carrying the new MAC-key confirm zeroizes the sender's old keys, and the deadline and u64::MAX hard limit close rather than overlap or wrap.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-11-25T01:01:03Z
- **Completed:** 2026-11-25T01:08:32Z
- **Tasks:** 2
- **Files modified:** 4 (3 code/test files plus the pin ledger)

## Accomplishments

- `Session::rekey_due`, `Session::rekey_step`, `Session::poll`, `Session::superseded_keys`, `transport::Superseded` and `transport::SEQ_HARD_LIMIT`; the in-band exchange runs sealed through the existing `SessionExchange` seam and chains a fresh `Transcript` from the previous generation's hash.
- Strict per-direction cutover: `SendHalf::marker_and_switch` seals the marker as the last old-key packet, moves the old keys into a superseded slot (`Held`), installs the new keys, resets the counter to 0 and stores the deadline in one lock scope.
- The receiver answers a marker with an ACK whose confirm is `rekey_confirm(g + 1)` under the new receive-direction MAC key; the sender verifies it (constant-time inside `crypto`) and zeroizes its old keys.
- The deadline is caller-driven only: `poll(now)`, `seal` and `open` close the waiting sender when `now >= ack_deadline`; the transport names no clock, no tick hook and no `Instant`.
- The counter never wraps: sequence `u64::MAX` is never sealed, and `Handshake::finish_with_send_seq` is the `#[doc(hidden)]` host-only constructor that proves it, with `TransportConfig` unchanged.
- The 13 `aead_transport_rekey_*` pins pass, including the independent wire pin that re-derives generation 1 from the recorded plain and sealed frames and checks both directions' MACs and confirm values.

## Task Commits

Each task was committed atomically (RED then GREEN):

1. **Task 1 RED: in-band re-key and marker cutover pins** - `746cc94` (test)
2. **Task 1 GREEN: exchange and per-direction marker cutover** - `6fb12d9` (feat)
3. **Task 2 RED: acknowledgement, deadline and counter-limit pins** - `4638bf8` (test)
4. **Task 2 GREEN: ACK confirm, deadline and hard counter limit** - `d409f4b` (feat)

## Files Created/Modified

- `port/src/chan/transport.rs` - the re-key state machine: `Rekey`, the exchange driver, `rekey_due`/`rekey_step`/`poll`/`superseded_keys`, `Superseded`, `SEQ_HARD_LIMIT`, `marker_and_switch`, the deadline checks and `finish_with_send_seq`
- `port/tests/common/aead_link.rs` - keepable seams (`connect_with_kept`/`connect_with_kept_config`), `config_with_rekey`, `rekey()` forwarding on both ends, `AeadLink::wire_log`, the `withhold_rekey_ack` drop and `TestExchange::runs`
- `port/tests/aead_transport.rs` - the 13 re-key pins and the test-side key/body derivation helpers
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-08.md` - the pin ledger (`transport-rekey-cutover`, `transport-rekey-ack`)

## Decisions Made

- Both directions derive generation g + 1 from one exchange, but each end cuts over only its own send direction on its own marker; the receiver of each marker answers in the opposite direction. This makes the two directions independent and keeps the ACK's confirm tied to the direction actually cut over.
- The sender's old keys live inside `SendState` so the marker, switch and counter reset are atomic under the send lock; the receiver's old keys live in a single `Session::rekey` slot.
- A server that reaches the threshold first sends a `REQUEST`; the exchange is always client-driven because `Tauth` is a client message.
- The deadline and the hard limit are added as dormant scaffolding in Task 1 and stubbed in Task 2 RED, so the Task 2 pins fail on assertions rather than compile errors (D-03, RESEARCH Pitfall 11).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 1 tests consumed the acknowledgement queue**
- **Found during:** Task 2 RED
- **Issue:** Enabling the marker acknowledgement (a Task 2 behavior) put an s2c ACK ahead of the s2c packets that two Task 1 pins (`directions_cut_over_independently`, `outstanding_requests_complete`) drained, so their MAC/tag assertions saw the ACK bytes.
- **Fix:** Both pins now open and accept the server's c2s ACK before the next s2c packet, which is what a real caller does.
- **Files modified:** `port/tests/aead_transport.rs`
- **Verification:** All 13 re-key pins and the full 35-pin transport file pass at Task 2 GREEN.
- **Committed in:** `d409f4b` (Task 2 GREEN)

### Plan-structure note

- Task 1 GREEN deliberately kept the acknowledgement disabled (as the plan's Task 1 text says), but it already carried the Task 2 plumbing (the deadline field, `poll`, `SEQ_HARD_LIMIT`, `finish_with_send_seq`) as dormant code. Task 2 RED stubbed those and Task 2 GREEN completed them. No behaviour beyond Task 1 was observable at the Task 1 GREEN commit.

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** The fix is test-only and reflects the real wire order; no scope creep.

## Issues Encountered

None beyond the ACK-queue ordering above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- WIRE-04 holds in full on the host: strict counters, in-band re-key before the limit, an acknowledged cutover with a deadline, and no wrap.
- **For Phase 6:** nothing in the kernel calls `poll(now)` yet. A live connection owner must drive `Session::poll(now)` periodically or the acknowledgement deadline never fires; this is recorded for the day a live mount exists (D-23, F4).
- Plan 04-13 can cite `aead_transport_rekey_wire_keys_and_confirm_equal_test_side_derivation` for RFC sections 3.2 and 11.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED
