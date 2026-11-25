---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 16
subsystem: kernel-transport-crypto
tags: [9p2000-aead, sequence-nonce, replay, splice, reflection, mcslock, send-order, fail-closed]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-07)
    provides: "port::chan::transport::{Session, Handshake, fail()}, the in-memory AeadLink fixture with replay_last/reflect knobs, the sealed engine and the 14 transport pins"
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "crypto::session::{DirectionKeys, SessionKeys, derive}; port::mcslock::Lock"
provides:
  - "the strict per-direction sequence check in Session::open, after the MAC and before any other header field (P7 accepted, WIRE-04)"
  - "port::chan::transport::SendHalf (re-exported from port::chan): Seal/write/advance under one mcslock::Lock, so concurrent senders stay in counter order (WIRE-04, D-21)"
  - "Session::seal shares SendHalf's lock and counter; fail/teardown/keys_zeroized reach the send keys through it (WIRE-03, D-15)"
  - "the sequence (2), splice (3) and send_order (3) pins; 22 transport pins total"
affects: [04-08 re-key (its pending/superseded slots join fail through the same lock), phase 05 page reads, phase-end code review of port/src]

# Actuals (#2632), on the plan's estimateTokens scale (chars/4 over the realized diff).
actuals:
  tokens: 9553
  tasks: 2
  commits: 6
  plan_head_before: 2e8e4dd4bb5c5fe73c5fcf99ed1e1edfc16959ff
  plan_head_after: 34025c4ff20dfe63fdea09acc4b4b7d708bcecf9

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "receive order is now: frame by Size -> verify the MAC -> compare SeqNonce with the expected next value -> check the header -> parse TLVs -> decrypt -> Verified"
    - "one per-direction send lock: the seal, the whole-packet write and the counter advance happen in one Lock<SendState> scope, so wire order equals counter order"
    - "Session::seal is the single-owner door through the same lock; SendHalf::send is the shared door; they cannot disagree about the counter"
    - "fail() zeroizes the send keys through the lock and the receive keys directly, then resets recv_seq; is_closed/keys_zeroized read the locked send state"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-16.md
  modified:
    - port/src/chan/transport.rs
    - port/src/chan.rs
    - port/tests/common/aead_link.rs
    - port/tests/aead_transport.rs

key-decisions:
  - "P7 accepted therefore implemented as fail-closed: a repeated, skipped or earlier sequence number closes the connection through fail() and zeroizes the keys, and the packet is never handed to the handler"
  - "The sequence comparison sits immediately after the MAC verify and before check_header, satisfying the prohibition that it is never made before the MAC"
  - "SendHalf holds Lock<SendState{keys, seq, closed}>; Session::seal delegates to the same lock so one counter serves both single-owner and shared callers"
  - "A write that does not land whole closes the direction, because a counter consumed by a packet the peer never saw would desynchronise the stream"
  - "The contention pin uses std::thread::scope over &SendHalf rather than an Arc: Lock makes SendHalf Sync, so no note-tag fallback was needed (NOTE transport-send-order not used)"
  - "The clippy too_many_arguments allowance is applied to SendHalf::send and seal_into because the plan mandates those signatures; the Session/SendHalf role fields were removed once the split keys made them unread"

patterns-established:
  - "a shared send half is safe to hand to two threads: Lock is Sync and the counter advance is inside the same guard as the write"
  - "the fixture's replay_last duplicates the sealed bytes on the wire, and reflect sends them back to the sender; both are pure test-side taps and reach no production code"

requirements-completed: [WIRE-04, WIRE-11]

coverage:
  - id: D1
    description: "Strict per-direction sequence numbers: 0,1,2 rise by one, and a repeat, a skip or an earlier value is rejected after the MAC verifies with the P7 outcome (closed and zeroized)"
    requirement: WIRE-04
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_sequence_starts_at_zero_and_increments"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_sequence_rejects_repeat_skip_and_earlier"
        status: pass
    human_judgment: false
  - id: D2
    description: "A replayed captured Twrite is not handed to the handler, and an edited cleartext FidHandle, a cross-connection packet and a reflected packet each fail the MAC"
    requirement: WIRE-11
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_splice_replayed_twrite_is_dropped"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_splice_edited_fidhandle_fails_mac"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_splice_cross_connection_and_reflection_fail"
        status: pass
    human_judgment: false
  - id: D3
    description: "Two threads sending through one shared SendHalf put sequence numbers 0 to 399 on the wire with no gap or repeat; Session::seal and SendHalf::send share the counter; a closed session writes nothing and stays zeroized"
    requirement: WIRE-04
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_send_order_under_contention"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_send_order_shares_the_session_counter"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_send_order_closed_session_refuses"
        status: pass
    human_judgment: false

# Metrics
duration: 4 min committed span
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 16: Strict sequence numbers, the replay and splice pins, ordered sending Summary

**A sealed connection now refuses a repeated, skipped or earlier `SeqNonce` right after the MAC verifies (P7: close and zeroize), and sealing plus writing sit under one per-direction lock so concurrent senders cannot reorder the wire.**

## Performance

- **Duration:** 4 min committed span (both RED/GREEN cycles, two `cargo xtask test` runs, `check` and `clippy`)
- **Started:** 2026-11-25T00:55:25Z
- **Completed:** 2026-11-25T00:59:37Z
- **Tasks:** 2
- **Files modified/created:** 5 (2 source, 1 fixture, 1 test file, 1 ledger)

## Accomplishments

- `Session::open` compares the packet's `SeqNonce` with the one value the receive direction expects, immediately after the MAC verifies and before `wire::check_header`; a repeat, a skip or an earlier value closes the connection through the one `fail()` path and zeroizes both directions (P7 accepted, WIRE-04).
- The replay (`replay_last`) and reflection (`reflect`) knobs are wired in the in-memory fixture; the replay pin proves a captured `Twrite` delivered twice is not handed to the handler, and the reflection pin proves a packet sent back to its sender fails the MAC.
- The edited-`FidHandle`, cross-connection and reflection cases need no new production code: D-07 and D-15 already make them fail the MAC with the decode counter unchanged, and the pins now hold that.
- The sending side is `SendHalf` over `mcslock::Lock<SendState>`; `send` seals, writes the whole packet and advances the counter inside one lock scope, and `Session::seal` shares the same lock and counter for single-owner use (D-21).
- `fail()`, `teardown()` and `keys_zeroized()` reach the send keys through the lock, so a close still zeroizes them and the query still reads them (WIRE-03, D-15).
- 22 transport pins pass; `cargo xtask test` twice, `check` and `clippy` all exit 0; `port/src/devssl.rs` unchanged.

## Task Commits

Each task was committed atomically (TDD: RED then GREEN; the ledger ADVANCED line is a separate docs commit because a commit cannot record its own SHA):

1. **Task 1 RED: strict sequence and splice pins** - `43657ab` (test)
2. **Task 1 GREEN: strict sequence check after the MAC** - `f2091d7` (feat)
3. **Task 1 ledger ADVANCED** - `13e3741` (docs)
4. **Task 2 RED: ordered sending under one lock** - `3f40bec` (test)
5. **Task 2 GREEN: SendHalf seals and sends under one per-direction lock** - `3f3a354` (feat)
6. **Task 2 ledger ADVANCED** - `34025c4` (docs)

**Plan metadata:** the SUMMARY commit that follows.

## Files Created/Modified

- `port/src/chan/transport.rs` - the strict sequence check in `Session::open`, `SendHalf`/`SendState`, `seal_into`, and the revised `Session` (locked send half, plain receive keys)
- `port/src/chan.rs` - re-exports `SendHalf`
- `port/tests/common/aead_link.rs` - wires `replay_last` and `reflect` into both ends' `send`
- `port/tests/aead_transport.rs` - the 2 sequence, 3 splice and 3 send_order pins (22 total with the earlier 14)
- `.planning/.../evidence/PINS-04-16.md` - the two pin ledgers (`transport-sequence-splice`, `transport-send-order`), RED/GREEN-1/GREEN-2/ADVANCED each

## Decisions Made

- **P7 implemented as accepted:** an unexpected sequence closes and zeroizes; nothing is dropped silently. The tests assert `is_closed()` and `keys_zeroized()`.
- **Sequence check placement:** after `mac.verify` and before `check_header`, so no header field other than `Size` (for framing) is used before verification.
- **SendHalf shares one counter with `Session::seal`:** both go through `Lock<SendState>`, so a session cannot have two counters.
- **A write that does not land whole closes the direction:** otherwise the counter would advance past a packet the peer never received.
- **Scoped threads, not `Arc`:** `Lock` makes `SendHalf` `Sync`, so `std::thread::scope` sharing `&SendHalf` proves the property directly; the plan's `NOTE transport-send-order` fallback was not needed.
- **RED shapes:** Task 1's RED ran the new pins against the 04-07 engine (no comparison). Task 2's RED landed `SendHalf` with a `send` that seals at a fixed sequence and advances nothing; both REDs failed on assertions, not compile errors (D-03, RESEARCH Pitfall 11).

## Deviations from Plan

None - plan executed as written.

The only non-plan text in the diff is mechanical: `#[allow(clippy::too_many_arguments)]` on `SendHalf::send` and `seal_into` (the plan mandates those signatures and clippy warns at 8 args), and the now-unread `role` fields were removed from `Session`/`SendHalf` during the GREEN cleanup. No wire byte, API name or test behaviour changed.

## Issues Encountered

None. `clippy` exits 0; the only `port/src/chan/transport.rs` warnings that appeared mid-GREEN (unread `role`, `too_many_arguments`, a `match` that could be `?`) were resolved before the GREEN commit.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The strict sequence half of WIRE-04 and WIRE-11's anti-splicing pins hold on the host; ROADMAP Phase 4 success criterion 2's second half is covered with plan 04-07.
- Plan 04-08's re-key can join `fail()` through the same `Lock<SendState>` (the pending/superseded slots will sit beside the live keys).
- `port/src/devssl.rs` stays unchanged; the `port/src` diff review is deferred to phase end (D-24).

## Self-Check: PASSED
