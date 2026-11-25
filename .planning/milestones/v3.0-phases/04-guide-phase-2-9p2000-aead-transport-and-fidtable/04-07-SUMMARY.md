---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "07"
subsystem: kernel-transport-crypto
tags: [9p2000-aead, handshake, transcript, exchange-seam, chacha20, blake3, zeroize, fail-closed, in-memory-link]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "crypto::session (derive, DirectionKeys, SessionKeys, PacketMac, Transcript, is_zeroized/zeroize); crypto::page::chacha20_xor; crypto::key::Key"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-02)
    provides: "the sealed-packet tracer: port::chan::{seal_packet, open_packet}, the 28-byte header and the packet MAC"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-03)
    provides: "crypto::session::Transcript with chain/hash, SessionKeys::zeroize/is_zeroized, DirectionKeys::is_zeroized"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-05)
    provides: "msg::encode_body/decode_body, DataLayout and msg::data_layout for the D-09 data bodies and the P5 keystream placement"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-06)
    provides: "tlv::Exts, tlv::encode and tlv::parse for the cleartext, authenticated TLV area"
provides:
  - "port::chan::transport — Role, TransportConfig, ExchangeStep, SessionExchange, Closed, Handshake, Session"
  - "Session::{seal, open, frame_len, receive, teardown, is_closed, keys_zeroized}; Handshake::{new, absorb_plain, finish}"
  - "the plain-framed handshake: Tversion/Rversion, Tauth/Rauth and one auth-fid Twrite/Rwrite/Tread/Rread round, all absorbed into the transcript"
  - "one fail-closed path: fail() zeroizes both directions' keys and both counters and closes; frame_len/receive/open route every rejection through it"
  - "port/tests/common/{mod.rs,aead_link.rs} — the in-memory duplex AeadLink, TestExchange, connect/connect_with, ClientEnd, ServerEnd, knobs, Stage and decode counter"
  - "the handshake (5), wire (2) and fail-closed (5) pins, with the two plan 04-02 tracer pins unchanged"
affects: [04-08 re-key (pending/superseded key slots join fail()), 04-16 strict sequence and splice pins, 04-17 test-9p, phase 05 page reads at these block counters]

# Actuals (#2632): the same estimateTokens scale as the plan's `estimate`.
actuals:
  tokens: 16619
  tasks: 2
  commits: 6
  plan_head_before: 0d606a90ecde8b7aebf69f0f3eaaf1e50caae229
  plan_head_after: e89fec1d2536fe8b2b57cfcd1414710367350cc7

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "one connection = one 32-byte session secret; both ends derive c2s/s2c from it, so a test hands the same secret to both ends"
    - "transcript binding: every plain-framed message is absorbed whole, sent or received, in wire order, before finish derives the keys (D-17)"
    - "the exchange seam hands the transport a Key and never reveals its origin (D-16); the only SessionExchange implementations live under port/tests"
    - "receive order: frame_len reads Size only; receive bounds with Unverified::new; open verifies the MAC, then reads seq, checks the header, parses the TLVs, decrypts, and only then builds Verified"
    - "a single private fail() is the only close path besides teardown, which calls it; keys_zeroized reads the stored keys, never the flag"
    - "P5 placement: parameters from block counter 0, data page i from 1 + 64 * i, a trailing partial page from 1 + 64 * pages"

key-files:
  created:
    - port/tests/common/mod.rs
    - port/tests/common/aead_link.rs
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-07.md
  modified:
    - port/src/chan/transport.rs
    - port/src/chan.rs
    - port/src/chan/wire.rs
    - port/tests/aead_transport.rs

key-decisions:
  - "One connection's two ends share the exchange's 32-byte secret (D-16): the tests hand the same Key to the client and server; the cross-connection pin gives two connections different secrets"
  - "Session::finish records whether both Tauth and Rauth were absorbed and seal/open/receive refuse before that; the wire pins build their session from exactly those two fixed messages"
  - "Session holds SessionKeys (both directions) and Role selects which direction seals; fail() therefore zeroizes send and receive in one SessionKeys::zeroize call"
  - "seal's only error type is Closed, so a packet over msize, a TLV area over 65535 bytes or a keystream overflow also closes the session rather than leaving it half-usable"
  - "Unverified::take is crate-private so Session::open, whose mandated signature takes Unverified, can verify and decrypt in place; the public surface still exposes only len()"
  - "the RED commits carry the real signatures with stale bodies: Task 1's seal/open were signature-carrying stubs and finish zeroed the keys; Task 2's fail/teardown set the closed flag only and frame_len/receive did not yet call fail()"
  - "the every-byte pin walks Size byte 1 first so its high bits (Size past msize) exercise the frame_len close, and a lower bit of byte 0 exercises the receive close"

patterns-established:
  - "in-memory duplex link: two byte queues, a fake clock and public knobs; the ends only ever frame through Session::frame_len and deliver through Session::receive"
  - "an independent wire pin holds its own DirectionKeys, derived with Transcript/derive in the test, and touches the sealed bytes with keystream and mac() only"
  - "a test-side sealer (seal_bypass) builds a valid MAC over arbitrary header and TLV bytes to pin the post-MAC checks"

requirements-completed: [WIRE-03, WIRE-05, WIRE-11]

coverage:
  - id: D1
    description: "The plain-framed handshake with a transcript-bound derivation and the SessionExchange seam: matching keys, an empty exchange, refusal before auth, an altered Rversion failing the first packet, and per-connection keys"
    requirement: WIRE-05
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_handshake_derives_matching_keys"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_handshake_with_empty_exchange"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_handshake_refuses_attach_before_auth"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_handshake_altered_rversion_fails_first_packet"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_handshake_keys_are_per_connection"
        status: pass
    human_judgment: false
  - id: D2
    description: "The sealed wire bytes: parameters at block counter 0, data page i at 1 + 64 * i, and the tag as the MAC over every byte before it, all computed in the test without the transport"
    requirement: WIRE-03
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_wire_data_pages_decrypt_alone_at_their_block_counter"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_wire_tag_is_packet_mac_of_all_bytes_before_it"
        status: pass
    human_judgment: false
  - id: D3
    description: "Failing closed: any altered authenticated byte, a framing rejection, a short slice or a malformed-after-MAC packet closes the session and zeroizes both directions' keys with nothing decoded; teardown zeroizes and Tclunk alone does not"
    requirement: WIRE-03
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_mac_failure_every_byte"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_mac_failure_closes_both_directions"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_mac_failure_zeroizes"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_teardown_zeroizes"
        status: pass
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_malformed_after_mac_closes"
        status: pass
    human_judgment: false
  - id: D4
    description: "The in-memory link fixture (port/tests/common/mod.rs, AeadLink, TestExchange, connect/connect_with, ClientEnd, ServerEnd, Stage, decode counter) that every transport pin above runs on"
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_handshake_derives_matching_keys (runs through common::aead_link)"
        status: pass
    human_judgment: false

# Metrics
duration: 4min committed span (~20 min including reading and both RED/GREEN cycles)
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 07: The sealed transport — handshake and failing closed Summary

**Plain-framed `Tversion`/`Rversion`, `Tauth`/`Rauth` and auth-fid `Twrite`/`Tread` establish one 32-byte exchange secret into per-direction keys through a transcript-bound HKDF, and a single `fail()` zeroizes and closes on any altered authenticated byte, with nothing parsed.**

## Performance

- **Duration:** 4 min committed span (~20 min including reading and both RED/GREEN cycles)
- **Started:** 2026-11-25T00:49:41Z (first task commit)
- **Completed:** 2026-11-25T00:53:24Z
- **Tasks:** 2
- **Files modified:** 7 (4 code/test files modified, 3 created)

## Accomplishments

- `port/src/chan/transport.rs` is the connection engine: `Role`, `TransportConfig`, `ExchangeStep`, `SessionExchange`, `Closed`, `Handshake` and `Session`.
- The handshake absorbs every plain-framed message whole, sent or received, in wire order into the unkeyed BLAKE3 transcript; `finish` derives the keys from that transcript, so an altered `Rversion` fails the first sealed packet (D-17).
- `Session::seal` writes the header, the cleartext TLV area, the body encrypted with parameters at block 0 and data page `i` at `1 + 64 * i` (P5), and the MAC over `[0, size - 32)`.
- `Session::open` verifies the MAC before reading any field but `Size`, then reads the sequence, checks the header, parses the TLVs, decrypts and builds `Verified`; every rejection goes through the one private `fail()`.
- `fail()` zeroizes both directions' keys through `SessionKeys::zeroize`, zeroes both counters and closes; `teardown()` takes the same path; `keys_zeroized()` reads the stored keys, not the flag.
- `Session::frame_len` and `Session::receive` are the raw-receive entry points, and the fixture's two ends frame every sealed packet through them.
- `port/tests/common/aead_link.rs` is the phase's first shared integration-test module: in-memory duplex link, fake clock, public knobs, `TestExchange`, `connect`, and a test-side sealer.
- 14 transport pins pass (the two plan 04-02 tracer pins unchanged); `cargo xtask test`, `check` and `clippy` all exit 0.

## Task Commits

Each task was committed atomically (TDD: RED then GREEN, with the ADVANCED ledger line in a separate docs commit because a commit cannot record its own SHA):

1. **Task 1 RED: the plain-framed handshake and exchange seam** - `752cee5` (test)
2. **Task 1 GREEN: handshake transcript and session establishment** - `b59003a` (feat)
3. **Task 1 ledger ADVANCED** - `bed5c6b` (docs)
4. **Task 2 RED: failing closed on MAC failure** - `78e68db` (test)
5. **Task 2 GREEN: fail closed and zeroize on any receive failure** - `32b2105` (feat)
6. **Task 2 ledger ADVANCED** - `e89fec1` (docs)

**Plan metadata:** see the SUMMARY commit that follows.

## Files Created/Modified

- `port/src/chan/transport.rs` - the connection engine: handshake, session, seal/open, frame_len/receive, the single fail-closed path, and the tracer wrappers kept intact
- `port/src/chan.rs` - re-exports the new transport names alongside `seal_packet`/`open_packet`
- `port/src/chan/wire.rs` - one crate-private `Unverified::take` so `Session::open` can verify and decrypt in place (see Deviations)
- `port/tests/common/mod.rs` - declares `pub mod aead_link;` so Cargo does not build it as its own test target
- `port/tests/common/aead_link.rs` - in-memory duplex link, fake clock, knobs, `TestExchange`, `connect`/`connect_with`, `ClientEnd`, `ServerEnd`, `Stage`, decode counter, `seal_bypass`
- `port/tests/aead_transport.rs` - the 5 handshake pins, 2 independent wire pins, 5 fail-closed pins, and the 2 unchanged tracer pins
- `.planning/.../evidence/PINS-04-07.md` - the two pin ledgers (`transport-handshake`, `transport-fail-closed`), RED/GREEN-1/GREEN-2/ADVANCED each

## Decisions Made

- **One secret per connection.** Both ends of a connection derive from the same exchange-produced 32-byte secret; the tests therefore hand the same `Key` to the client and server. The cross-connection pin gives two connections different secrets. (My first GREEN attempt used two different secrets and the handshake pins failed on the server MAC — corrected before the GREEN commit.)
- **Authentication gate.** `Handshake::finish` records whether both `Tauth` and `Rauth` were absorbed; `seal`, `open` and `receive` refuse before that. The wire pins build their session from exactly those two fixed messages, which is also what lets them check the transcript independently.
- **`seal`'s only error is `Closed`.** A packet over msize, a TLV area too long for the `u16` field, or a keystream overflow closes the session rather than leaving a half-usable connection; the signature the plan mandates has no other error type.
- **Page counters follow P5 exactly**: parameters from block 0, page `i` from `1 + 64 * i`; a trailing partial page (never produced by `msg::encode_body`, which pads) would take `1 + 64 * pages`.
- **`fail` is the only close path**; `teardown` calls it. `keys_zeroized` reads `SessionKeys::is_zeroized` over the stored send and receive slots, so a flag-only `fail` would answer `false`.
- **RED shapes:** Task 1's RED carried the real signatures with `seal`/`open` stubs and a key-zeroing `finish`; Task 2's RED carried `fail`/`teardown` as flag-only and left `frame_len`/`receive` not closing on a framing rejection. Every RED failure was behavioural (D-03, Pitfall 11).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added a crate-private `Unverified::take` in `wire.rs`**
- **Found during:** Task 1 GREEN (implementing `Session::open`)
- **Issue:** The mandated `Session::open(packet: wire::Unverified<'a>)` signature must verify the MAC and decrypt the body in place, but `Unverified` exposed only `len()` and `wire.rs` was not in the plan's `files_modified`. Without a byte accessor the signature cannot be implemented.
- **Fix:** Added `pub(crate) fn take(self) -> &'a mut [u8]` to `Unverified`, documented as crate-private and used only by `chan::transport` after `new` has bounded the packet; the public surface still exposes nothing but `len()`.
- **Files modified:** `port/src/chan/wire.rs`
- **Verification:** `cargo xtask test`, `check` and `clippy` exit 0; all 14 transport pins pass; the plan 04-02 tracer pins are unchanged.
- **Committed in:** `b59003a` (Task 1 GREEN)

**2. [Rule 1 - Bug] The client end received from the wrong queue**
- **Found during:** Task 1 GREEN (first run of `aead_transport_handshake_derives_matching_keys`)
- **Issue:** `ClientEnd::recv` drained `c2s` (the queue for bytes the client wrote) instead of `s2c` (bytes for the client), so the sealed `Rattach` was never seen.
- **Fix:** Changed `ClientEnd::recv` to drain `s2c`; `ServerEnd::recv` correctly drains `c2s`.
- **Files modified:** `port/tests/common/aead_link.rs`
- **Verification:** the full handshake pin now passes both directions.
- **Committed in:** `b59003a` (Task 1 GREEN)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both were necessary to reach the plan's own behaviour; no scope creep and no wire-contract change.

## Issues Encountered

None beyond the two auto-fixes above.

## Boundaries this plan deliberately leaves open

- **No replay protection yet.** The strict next-in-sequence comparison is deliberately absent from `Session::open`; plan 04-16 adds it at exactly the step that reads `SeqNonce`. This plan's result must not be read as replay protection.
- **A larger `Size` inside msize closes late.** A `Size` altered to a larger value that is still within msize is undetectable until that many bytes have arrived, at which point the MAC fails; the transport has no read timeout this phase, so bounding a stalled frame is the caller's job.
- **No live authenticated exchange.** The `SessionExchange` seam is injected; the real authenticated exchange (Noise vs DP9IK) is later phases' work, and no live connection between two real machines comes out of this phase.
- **No re-key here.** The `pending`/`superseded` key slots and the `now`-driven timing are plan 04-08's; `fail()`'s doc says those slots will join it.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The engine, the in-memory link and the fail-closed path are ready for plan 04-16's strict sequence, replay, reflection and splice pins on the same files, and for plan 04-08's re-key.
- Phase 5 can read data pages at the pinned counters (`1 + 64 * i`) knowing the placement is pinned against independently computed bytes.
- `port/src/devssl.rs` is unchanged.

## Self-Check: PASSED

- `port/src/chan/transport.rs`, `port/src/chan.rs`, `port/src/chan/wire.rs`, `port/tests/common/mod.rs`, `port/tests/common/aead_link.rs`, `port/tests/aead_transport.rs` and `evidence/PINS-04-07.md` all exist.
- Commits `752cee5`, `b59003a`, `bed5c6b`, `78e68db`, `32b2105`, `e89fec1` are ancestors of HEAD.
- `cargo xtask test` (twice), `cargo xtask check` and `cargo xtask clippy` exit 0; `cargo test -p port --test aead_transport` is 14 passed.
- The ledger has exactly 8 `PIN (transport-handshake|transport-fail-closed) …` lines.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*
