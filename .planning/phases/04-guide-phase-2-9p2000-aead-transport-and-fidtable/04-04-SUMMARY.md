---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "04"
subsystem: kernel-transport-wire-codec
tags: [9p2000-aead, wire-header, opcode-registry, typestate, little-endian, tdd]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-02)
    provides: "port::chan constants, seal_packet/open_packet, crypto::session DirectionKeys"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-01)
    provides: "the eight owner-accepted wire decisions (P1, P3, D-28), the header length correction"
provides:
  - "port::chan::wire — Header (encode/decode), frame_size, packet_len, the 100-139 opcode registry, opcode_name, is_t_message, Unverified, Verified, check_header"
  - "port::chan::{PAGE_LEN, MIN_PACKET_LEN, EXT_DEADLINE, EXT_CAP_TOKEN, EXT_QID_VOLATILITY, EXT_STREAM_PRIO, EXT_OBINARY, EXT_STATELESS_CAP, EXT_KNOWN}"
  - "port::chan::{wire, tlv, msg, transport} submodules; port/src/chan/transport.rs holds seal/open"
  - "the wire-header and wire-registry pins (RED, GREEN-1, GREEN-2, ADVANCED) in evidence/PINS-04-04.md"
affects: [04-05 message bodies, 04-06 TLVs, 04-13 RFC registry amendment, 04-17 test-9p]

actuals:
  tokens: 10684
  tasks: 2
  commits: 6
  plan_head_before: f2455ef104611209b24b3a1c29006e93229b58e1
  plan_head_after: bce54ee28c7140bc3938c70e9f01a3ec4dd340b8

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "the header is written and read field by field at pinned offsets with to_le_bytes/from_le_bytes; no packed struct, no transmute"
    - "Size is the only pre-MAC field, bounded by frame_size before any body byte is read"
    - "Unverified exposes only len(); Verified has private fields and one crate-private constructor called after the MAC"
    - "check_header runs only after verification and fails closed on Pad, unknown opcode, ExtMask and TLV-length inconsistencies"
    - "packet_len checks every length sum, the u32 Size field and the negotiated msize with no silent narrowing"

key-files:
  created:
    - port/src/chan/wire.rs
    - port/src/chan/transport.rs
    - port/src/chan/tlv.rs
    - port/src/chan/msg.rs
    - port/tests/aead_wire.rs
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-04.md
  modified:
    - port/src/chan.rs

key-decisions:
  - "The base opcode set is fcall.h's 100 to 127 with 106 (Terror) absent, plus Rerror 107 and Tflush/Rflush 108/109 (D-28)"
  - "The new pairs 130 to 137 take the RFC section 6.1 names; 138/139 are Trekey/Rrekey (P3)"
  - "is_t_message is true exactly for a known, even opcode; an unknown value is never a T-message"
  - "open_packet copies the read-only caller slice once so it can decrypt in place; Unverified::new applies the bounds to the copy before the MAC"
  - "Verified::new_verified is pub(crate) and called from transport only; the typestate is asserted on source (no port --doc leg exists)"
  - "EXT_KNOWN is 0x003F, the OR of the six P1 mask bits, and a test pins that identity"

patterns-established:
  - "TDD per wire pin: RED with compiling wrong bodies, GREEN twice under cargo xtask test, then a docs commit that advances the pin"
  - "Hand-written wire vectors in tests, never produced by the encoder under test"
  - "Typestate proof by crate-private constructor plus a source grep in the plan's verify, not a compile_fail doctest"

requirements-completed: [WIRE-01, WIRE-06]

coverage:
  - id: D1
    description: "The canonical 28-byte header: eight field offsets, little-endian encode/decode, round trip and the framing bounds (59 rejected, 60 and msize accepted, msize + 1 rejected)"
    requirement: "WIRE-01"
    verification:
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_header_offsets_and_length"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_header_hand_vector_decodes"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_header_roundtrip"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_header_size_bounds"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask test (the port leg runs the two tracer tests through the refactored codec)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Opcode registry pinned as a table: 100-105, 107-127, 130-137, 138/139 known; 106, 128, 129 and 140+ rejected; is_t_message true exactly for even known opcodes"
    requirement: "WIRE-06"
    verification:
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_registry_table_is_pinned"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_registry_rejects_terror_and_gaps"
        status: pass
    human_judgment: false
  - id: D3
    description: "A Verified packet only comes from the transport's MAC check: private fields, crate-private new_verified called from transport.rs only"
    requirement: "WIRE-03"
    verification:
      - kind: other
        ref: "grep -c 'pub(crate) fn new_verified' port/src/chan/wire.rs == 1 (Task 2 verify)"
        status: pass
      - kind: other
        ref: "grep -rn new_verified port/src => wire.rs definition + transport.rs call only"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_header_rules_after_verification"
        status: pass
    human_judgment: false
  - id: D4
    description: "Post-verification header rules: zero Pad, known opcode, ExtMask within EXT_KNOWN, TLV length consistent with the mask, multiple of 8, inside the packet; capability bit requires an all-zero handle (D-12)"
    requirement: "WIRE-01"
    verification:
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_header_rules_after_verification"
        status: pass
    human_judgment: false
  - id: D5
    description: "Both pins carry RED, GREEN-1, GREEN-2 and ADVANCED lines in the phase evidence"
    verification:
      - kind: other
        ref: "grep -cE '^PIN (wire-header|wire-registry) (RED|GREEN-1|GREEN-2|ADVANCED)' evidence/PINS-04-04.md == 8"
        status: pass
    human_judgment: false

# Metrics
duration: 5min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 04: The 28-byte header and the opcode registry Summary

**The canonical 28-byte 9P2000.aead header with a typed verify-before-parse packet split (`Unverified` → `Verified`) and the full 100-139 opcode registry, all pinned by hand-written little-endian wire vectors.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-11-25T00:25:02Z
- **Completed:** 2026-11-25T00:30:01Z
- **Tasks:** 2 (four TDD steps across two pins)
- **Files modified:** 7 (6 created, 1 rewritten)

## Accomplishments

- `port::chan` is now a parent module (`wire`, `tlv`, `msg`, `transport`) that keeps every constant at its root and re-exports `seal_packet`/`open_packet`, so `port/tests/aead_transport.rs` compiles and passes unchanged.
- `port/src/chan/wire.rs` pins the header field by field: `Header::encode`/`decode` place Size(0) Type(4) Tag(5) Pad(7) FidHandle(8) SeqNonce(16) ExtMask(24) ExtTLVs Length(26) little-endian, with a hand-written 28-byte vector whose fields all have distinct bytes.
- `frame_size` applies both bounds before any body byte is read (60 and the negotiated msize), rejecting 59, msize + 1 and an msize below 60; `packet_len` checks every length sum, the `u32` `Size` field and msize with no silent narrowing.
- The registry is a pinned table: fcall.h 100-127 with 106 absent, `Rerror`/`Tflush`/`Rflush` present (D-28), RFC section 6.1's 130-137, and P3's `Trekey`/`Rrekey` (138/139). `opcode_name` returns `None` for every other `u8`.
- `Unverified` exposes only `len()`; `Verified` has private fields and one `pub(crate) fn new_verified` called from `transport.rs` after `PacketMac::verify`. `check_header` then enforces Pad, opcode, `ExtMask`, TLV-length consistency and D-12's all-zero handle with the capability bit.
- `transport::open_packet` now runs the full receive order: bounds (`Unverified`) → MAC → sequence → `check_header` → decrypt body → `Verified`. The tracer's public return type is unchanged.

## Task Commits

Each TDD step was committed atomically:

1. **Task 1a: RED pin for the 28-byte header** - `66929bf` (test)
2. **Task 1b: the canonical 28-byte header and framing bounds** - `0e47e44` (feat)
3. **Task 1c: advance the wire-header pin** - `3079895` (docs)
4. **Task 2a: RED pin for the opcode registry and header rules** - `6909f3f` (test)
5. **Task 2b: opcode registry, `Unverified` and `Verified` packets, header rules** - `8c80b5f` (feat)
6. **Task 2c: advance the wire-registry pin** - `bce54ee` (docs)

**Plan metadata:** the SUMMARY commit follows (docs).

## Files Created/Modified

- `port/src/chan.rs` — parent module: submodule declarations, the re-exports, the pinned constants plus `PAGE_LEN`, `MIN_PACKET_LEN` and the six `ExtMask` bits with `EXT_KNOWN`.
- `port/src/chan/wire.rs` — `Header`, `frame_size`, `packet_len`, the opcode constants and registry, `Unverified`, `Verified`, `check_header`.
- `port/src/chan/transport.rs` — `seal_packet` and `open_packet` re-expressed through the codec, with the full receive order.
- `port/src/chan/tlv.rs` — doc-only module for plan 04-06.
- `port/src/chan/msg.rs` — doc-only module for plan 04-05.
- `port/tests/aead_wire.rs` — nine pins: six header pins and three registry/rule pins, every expected byte hand-written.
- `evidence/PINS-04-04.md` — eight PIN lines (two pins × RED/GREEN-1/GREEN-2/ADVANCED).

## Opcode names for 130 to 137 (for plans 04-05 and 04-13)

| Opcode | Name | | Opcode | Name |
|---|---|---|---|---|
| 130 | `Tcompound` | | 131 | `Rcompound` |
| 132 | `Tstream` | | 133 | `Rstream` |
| 134 | `Tlease_revoke` | | 135 | `Rlease_ack` |
| 136 | `Treclaim` | | 137 | `Rreclaim` |

P3's re-key pair is `Trekey` 138 / `Rrekey` 139.

## Decisions Made

- The registry name strings follow the RFC's spelling exactly (`Tlease_revoke`, `Rlease_ack`), so plans 04-05 and 04-13 can compare against the same table.
- `is_t_message` returns false for an unknown value rather than keying on parity alone: 128 or 140 must never read as a request.
- `open_packet` copies the caller's slice once (the public signature is read-only) and applies `Unverified::new` to the copy first, so the copy is bounded before anything else touches it.
- `Verified::new_verified` takes the already-decoded header and the whole packet, and bounds-checks both slices with `get(..)`, never indexing on a wire length.
- `check_header` treats an msize/TLV inconsistency as a protocol error (fail closed), matching RESEARCH Pattern 2: both ends agreed the dialect, so there is nothing to skip gracefully.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Clippy flagged the new modulo checks in `wire.rs`**
- **Found during:** Task 2 GREEN verification (`nix develop -c cargo xtask clippy`)
- **Issue:** `clippy::manual_is_multiple_of` warned on `typ % 2 == 0` and `ext_len % 8 != 0` in the new `port/src/chan/wire.rs`.
- **Fix:** Replaced them with `typ.is_multiple_of(2)` and `!ext_len.is_multiple_of(8)`; behaviour is identical.
- **Files modified:** `port/src/chan/wire.rs`
- **Verification:** `cargo xtask clippy` exits 0 with no warning naming `port/src/chan/`; `cargo xtask test` re-run twice green on the final code.
- **Committed in:** `8c80b5f`

---

**Total deviations:** 1 auto-fixed (1 bug/lint)
**Impact on plan:** No scope change; the wire bytes and every public signature are as planned.

## Issues Encountered

None that survived to a commit. The RED step for Task 1 first routed the moved tracer functions through the not-yet-implemented codec stubs; that was corrected before the RED commit so the tracer stayed green at RED (the plan's "move the tracer's two functions unchanged"), and the GREEN step then switched them to the codec.

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None. `port/src/chan/tlv.rs` and `port/src/chan/msg.rs` hold only module docs by design: they are the module declarations that keep this plan's file set disjoint from plans 04-05 (message bodies) and 04-06 (TLVs), which fill them in. They are not stubs of any deliverable this plan claims.

## Threat Flags

None — the new surface is exactly the plan's threat register (T-04-11 `frame_size` bounds, T-04-12 the crate-private `Verified`, T-04-13 the D-12 capability/handle rule), each with a passing pin above.

## Next Phase Readiness

- The header layout, the framing bounds, the registry and the post-verification rules are all pinned; plans 04-05 and 04-06 can build message bodies and TLVs on `Verified::{tlvs, body}` without redefining the header.
- `port/src/lib.rs` and `port/tests/aead_transport.rs` are untouched by this plan (`git diff` over the plan range shows zero lines), so the tracer's contract holds for the rest of the wave.
- `port/src/devmnt.rs` is untouched (acceptance criterion), and its private opcode constants were not widened.
- The TLV area is carried cleartext between the header and the ciphertext; plan 04-06 owns its record parser, and this plan only enforces that its length is consistent, a multiple of 8 and inside the packet.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- Every created file exists on disk (`port/src/chan/{wire,transport,tlv,msg}.rs`, `port/tests/aead_wire.rs`, `evidence/PINS-04-04.md`, this SUMMARY).
- All six plan commits are ancestors of HEAD (`66929bf`, `0e47e44`, `3079895`, `6909f3f`, `8c80b5f`, `bce54ee`).
- The pin ledger carries all eight lines (two pins x RED/GREEN-1/GREEN-2/ADVANCED).
- `cargo xtask test` (twice), `cargo xtask check` and `cargo xtask clippy` exit 0 on the final tree; nine `aead_wire_` tests and both tracer tests pass.
