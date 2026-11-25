---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "06"
subsystem: kernel-transport-wire-codec
tags: [9p2000-aead, tlv, extmask, zero-copy, tdd, wire]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-04)
    provides: "the six EXT_* bits, EXT_KNOWN, and the Verified packet's borrowed TLV area"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-01)
    provides: "the owner-accepted P1 record format and D-11, D-12, D-27"
provides:
  - "port::chan::tlv — Exts, QidVolatility, parse, encode, stream_prio_parts, TLV_HEADER_LEN and the six LEN_* constants"
affects: [04-07 transport TLV validation, 04-09 FidEntry/StatelessCap, 04-13 RFC registry and section 4.4 amendment]

actuals:
  tokens: 9123
  tasks: 2
  commits: 6
  plan_head_before: b636e0b79a2952063e42c208e8ccb9d11f9ac9da
  plan_head_after: c1d3945ca977b78b3d47ec3dd8bdd728dffb4a05

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "the TLV parser is a bounds-checked cursor over borrowed bytes; the 32-byte token and 24-byte capability are references into the area, so a parse owns no Vec and copies nothing"
    - "one distinct static error string per broken rule; there is no skip path for an unknown extension bit"
    - "each record is padded to a multiple of 8 bytes from the start of the area; the area starts at packet offset 28 and needs no alignment cast because every field is read with from_le_bytes"

key-files:
  created:
    - port/tests/aead_tlv.rs
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-06.md
  modified:
    - port/src/chan/tlv.rs

key-decisions:
  - "The record type byte is the ExtMask bit index (0 to 5), so encode derives it with trailing_zeros and parse requires it to match the bit being walked"
  - "parse validates the mask against EXT_KNOWN before reading any byte, so an unknown bit is fatal even with an empty area"
  - "The duplicate/out-of-order distinction compares the record type against the bit being walked: lower is a duplicate, higher is out of order, and a type whose bit is clear is a third rule"
  - "The volatility class is checked in the record's own byte and never related to dat::Qid's type byte (D-27)"
  - "encode appends padded records in ascending type order and documents that callers append to an 8-aligned area"

requirements-completed: [WIRE-02]

coverage:
  - id: D1
    description: "The six extension records round-trip in every one of the 64 mask combinations, with padded lengths 16, 40, 16, 8, 8 and 32"
    requirement: "WIRE-02"
    verification:
      - kind: unit
        ref: "port/tests/aead_tlv.rs#aead_tlv_each_extension_roundtrips"
        status: pass
      - kind: unit
        ref: "port/tests/aead_tlv.rs#aead_tlv_all_64_masks_roundtrip"
        status: pass
      - kind: unit
        ref: "port/tests/aead_tlv.rs#aead_tlv_mask_zero_is_empty_area"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask test (the port leg runs aead_tlv with every integration test)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The parser borrows the area and allocates nothing: a counting global allocator sees no allocation across a parse of the six-record area"
    requirement: "WIRE-02"
    verification:
      - kind: unit
        ref: "port/tests/aead_tlv.rs#aead_tlv_parse_allocates_nothing"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every malformed area is rejected with a distinct error: unknown bits, wrong order or duplicates, bad len/reserved/padding, a mask/area mismatch and a bad volatility class"
    requirement: "WIRE-02"
    verification:
      - kind: unit
        ref: "port/tests/aead_tlv.rs#aead_tlv_rejects_unknown_bit"
        status: pass
      - kind: unit
        ref: "port/tests/aead_tlv.rs#aead_tlv_rejects_wrong_order_and_duplicates"
        status: pass
      - kind: unit
        ref: "port/tests/aead_tlv.rs#aead_tlv_rejects_bad_len_reserved_and_padding"
        status: pass
      - kind: unit
        ref: "port/tests/aead_tlv.rs#aead_tlv_rejects_mask_area_mismatch"
        status: pass
      - kind: unit
        ref: "port/tests/aead_tlv.rs#aead_tlv_rejects_bad_volatility_class"
        status: pass
      - kind: unit
        ref: "port/tests/aead_tlv.rs#aead_tlv_record_ending_at_area_end_is_accepted"
        status: pass
    human_judgment: false
  - id: D4
    description: "The stream-priority value splits into its 2-bit tier and 30-bit credit"
    requirement: "WIRE-02"
    verification:
      - kind: unit
        ref: "port/tests/aead_tlv.rs#aead_tlv_stream_prio_is_two_plus_thirty"
        status: pass
    human_judgment: false

# Metrics
duration: 3min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 06: ExtMask and TLV records Summary

**The six extension records over 8-byte-aligned TLVs, parsed zero-copy into borrowed `Exts` and validated strictly, so every malformed area is a distinct `chan: tlv ...` protocol error.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-11-25T00:40:11Z
- **Completed:** 2026-11-25T00:44:03Z
- **Tasks:** 2 (four TDD steps across two pins)
- **Files modified:** 3 (1 filled in, 2 created)

## Accomplishments

- `port/src/chan/tlv.rs` is now the real parser: `parse(mask, area)` walks the `ExtMask` bits in ascending order and returns an `Exts` that borrows the 32-byte token and 24-byte capability from the area; `encode` appends the records in the same order; `stream_prio_parts` splits the 4-byte priority.
- Every one of the 64 mask combinations round-trips through `encode`/`parse`, with `Exts::encoded_len` equal to the area length and padded record lengths 16, 40, 16, 8, 8 and 32.
- `aead_tlv_parse_allocates_nothing` pins zero-copy with a per-thread counting `#[global_allocator]`: the allocation count is unchanged across a parse of the full six-record area.
- Strict validation rejects an unknown mask bit with no skip path, a record type outside `EXT_KNOWN`, a record type whose bit is clear, duplicates, out-of-order types, a wrong `len`, a nonzero reserved byte, nonzero padding, a short area and trailing bytes — each with its own error string.
- The volatility class is accepted only as one of 0x20, 0x40, 0x80 (D-27), in its own byte and independent of `dat::Qid`.
- `EXT_CAP_TOKEN` keeps its single 32-byte meaning and the capability travels only in the sixth-bit record (D-11, D-12).

## Task Commits

Each TDD step was committed atomically:

1. **Task 1a: RED pin for the six extension records** - `24ea225` (test)
2. **Task 1b: ExtMask and TLV records** - `f03c7a2` (feat)
3. **Task 1c: advance the tlv-roundtrip pin** - `f6cc579` (docs)
4. **Task 2a: RED pin for malformed TLV areas** - `90300f9` (test)
5. **Task 2b: strict TLV validation** - `f2de03c` (feat)
6. **Task 2c: advance the tlv-rejection pin** - `c1d3945` (docs)

**Plan metadata:** the SUMMARY commit follows (docs).

## Files Created/Modified

- `port/src/chan/tlv.rs` — the module doc, `TLV_HEADER_LEN`, the six `LEN_*` constants, `QidVolatility`, `Exts` (`mask`, `encoded_len`), `parse`, `encode`, `stream_prio_parts`.
- `port/tests/aead_tlv.rs` — eleven pins, the hand-written record vectors, the multi-record builder and a per-thread counting global allocator.
- `evidence/PINS-04-06.md` — eight PIN lines (`tlv-roundtrip` and `tlv-rejection`, each RED/GREEN-1/GREEN-2/ADVANCED).

## Decisions Made

- Used the record type byte as the `ExtMask` *bit index* (0 to 5), derived with `trailing_zeros` in `encode`; this keeps the wire byte and the mask bit in one place.
- `parse` checks `mask & !EXT_KNOWN` before touching the area, so an unknown bit is fatal even when the area is empty (T-04-18).
- Distinguished duplicate from out-of-order by comparing the record type with the bit being walked; a record whose type bit is clear gets its own error, so the three rules never share a message.
- Kept the 32-byte and 24-byte values as `&[u8; N]` references into the area, which is why `Exts` is `Copy` and the zero-allocation pin holds.
- Reworded one module-doc line to avoid the bare token `unsafe`, so the plan's "contains no `unsafe`" acceptance check is literal as well as true.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added a pin-ledger docs commit per task**
- **Found during:** Task 1 and Task 2 close-out
- **Issue:** The plan says "pin `tlv-roundtrip`" / "pin `tlv-rejection`" and its verify counts eight `PIN` lines, but the `ADVANCED` line must name the `feat` commit hash, which cannot exist inside that same commit. The action's short commit list named only the test and feat commits.
- **Fix:** Followed plan 04-04's established pattern: a `docs(04-06): advance the ... pin` commit after each GREEN-2 records the RED/GREEN-1/GREEN-2 evidence and the `ADVANCED` hash.
- **Files modified:** `evidence/PINS-04-06.md`
- **Verification:** `grep -cE '^PIN (tlv-roundtrip|tlv-rejection) (RED|GREEN-1|GREEN-2|ADVANCED)'` returns 8.
- **Committed in:** `f6cc579`, `c1d3945`

---

**Total deviations:** 1 auto-fixed (1 blocking/process)
**Impact on plan:** No wire or API change; only the two planned pin-ledger commits required to make the plan's own ADVANCED evidence and verify count possible.

## Issues Encountered

None that survived to a commit. The Task 1 parser intentionally trusted the record type, `len`, `reserved`, padding and exact consumption so Task 2's rejection tests had real RED failures; the Task 2 GREEN then tightened every rule.

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None.

## Threat Flags

None — the new surface is exactly the plan's threat register: T-04-17 (fixed length against bytes left, exact consumption), T-04-18 (mask subset of `EXT_KNOWN`, ascending order, no skip path) and T-04-19 (one of three volatility classes), each with a passing pin above.

## Next Phase Readiness

- `port::chan::tlv::{parse, encode}` is ready for plan 04-07 to call on `Verified::tlvs()` and close the connection on any error; `Exts` gives the transport the six decoded values without allocating.
- The 24-byte capability and its sixth-bit record are pinned, so plan 04-09's `FidEntry`/`StatelessCap` can reference the same layout and plan 04-13 can amend RFC section 4.4 to six bits.
- `nix develop -c cargo xtask test` (twice), `check` and `clippy` exit 0; eleven `aead_tlv_` tests pass.
- `port/src/dat.rs` is unchanged and `tlv.rs` names none of `QTDIR`, `QTAPPEND` or `QTEXCL` (D-27 held).

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- Every created/modified file exists on disk (`port/src/chan/tlv.rs`, `port/tests/aead_tlv.rs`, `evidence/PINS-04-06.md`, this SUMMARY).
- All six plan commits are ancestors of HEAD (`24ea225`, `f03c7a2`, `f6cc579`, `90300f9`, `f2de03c`, `c1d3945`).
- The pin ledger carries all eight lines (two pins x RED/GREEN-1/GREEN-2/ADVANCED).
- `cargo xtask test` (twice), `cargo xtask check` and `cargo xtask clippy` exit 0 on the final code tree; eleven `aead_tlv_` tests pass.
