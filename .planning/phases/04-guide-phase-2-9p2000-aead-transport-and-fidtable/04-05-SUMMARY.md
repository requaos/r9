---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "05"
subsystem: kernel-transport-wire-codec
tags: [9p2000-aead, message-bodies, wire-format, page-aligned-data, rekey, tdd]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-04)
    provides: "port::chan::wire opcode constants + PAGE_LEN; the Verified body split"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-01)
    provides: "the owner-accepted wire decisions P3, P4, P5 and D-28"
provides:
  - "port::chan::msg — Msg (27 base variants, 8 new-pair variants, Trekey/Rrekey)"
  - "msg::{opcode, encode_body, decode_body} and the D-09 two-part data body"
  - "msg::DataLayout { params_len, pages } and data_layout(typ, body)"
  - "msg::{MAXWELEM, MAX_PARAMS_LEN, REKEY_REQUEST, REKEY_MARKER, REKEY_ACK}"
  - "the ten aead_wire_messages_* pins and evidence/PINS-04-05.md"
affects: [04-06 TLVs (disjoint files), 04-13 RFC registry amendment, Phase 5 DMA ingress, Phase 6 message semantics]

actuals:
  tokens: 18083
  tasks: 2
  commits: 7
  plan_head_before: b636e0b79a2952063e42c208e8ccb9d11f9ac9da
  plan_head_after: 4b524640310916123faa150d8bd8ceeec9149c84

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "a private checked cursor (Cur) reads every field through get() and requires exact consumption (done)"
    - "every wire length is written with u16::try_from/u32::try_from that errors; no `as` cast narrows a length"
    - "data messages are parameters then zero padding to a whole number of PAGE_LEN pages; the decoder enforces count <= data_len < count + PAGE_LEN and zero padding"
    - "variable opaque tails (op_stream, result_stream, block_ranges) are `cur.rest()`, consumed and exact-checked"
    - "Msg derives Clone+Debug but implements PartialEq/Eq by hand because dat::Qid has no PartialEq"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-05.md
  modified:
    - port/src/chan/msg.rs
    - port/tests/aead_wire.rs

key-decisions:
  - "Msg PartialEq/Eq is hand-written; dat::Qid does not derive PartialEq and dat.rs is read-only to this plan"
  - "new-pair variants use CamelCase identifiers (TleaseRevoke, RleaseAck); opcode_name keeps the RFC spellings (Tlease_revoke, Rlease_ack)"
  - "data_layout uses fixed parameter lengths 16/4/16 for Twrite/Rread/Rstream and reports pages = data_len / PAGE_LEN"
  - "each pin carries RED, GREEN-1, GREEN-2 and ADVANCED lines (D-03), as in plan 04-04"

requirements-completed: [WIRE-06]

coverage:
  - id: D1
    description: "The 27 base 9P2000 bodies 100-127 (no 106) encode to hand-built bytes and decode to an equal Msg; strings are raw bytes with a 16-bit length; Twalk/Rwalk preserve order and reject 17 elements"
    requirement: "WIRE-06"
    verification:
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_messages_base_set_roundtrip"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_messages_base_set_covers_the_registry"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_messages_strings_are_raw_bytes"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_messages_walk_order_and_limits"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_messages_reject_truncated_and_trailing"
        status: pass
    human_judgment: false
  - id: D2
    description: "The new pairs 130-137 and Trekey/Rrekey 138/139 round-trip in the RFC 6.2 field order; the re-key body is 37 bytes and kind 3 is rejected"
    requirement: "WIRE-06"
    verification:
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_messages_new_pairs_roundtrip"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_messages_rekey_body"
        status: pass
    human_judgment: false
  - id: D3
    description: "Twrite/Rread/Rstream carry a parameter block then zero-padded 4096-byte pages with the true count; counts 0/1/4095/4096/4097 map to 0/1/1/1/2 pages and decode enforces the page and count bounds"
    requirement: "WIRE-06"
    verification:
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_messages_data_bodies_are_whole_pages"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_messages_data_bounds"
        status: pass
      - kind: unit
        ref: "port/tests/aead_wire.rs#aead_wire_messages_data_split_matches_layout"
        status: pass
    human_judgment: false
  - id: D4
    description: "The pin ledger records the Harvey oracle digests and both pins' RED/GREEN-1/GREEN-2/ADVANCED lines"
    verification:
      - kind: other
        ref: "grep -cE '^ORACLE harvey fcall\\.h=[0-9a-f]{64} convS2M\\.c=[0-9a-f]{64}$' evidence/PINS-04-05.md == 1"
        status: pass
      - kind: other
        ref: "grep -cE '^PIN (wire-messages-base|wire-messages-data) (RED|GREEN-1|GREEN-2|ADVANCED)' evidence/PINS-04-05.md == 8"
        status: pass
    human_judgment: false
  - id: D5
    description: "The transparency and safety prohibitions: 130-139 carry no behaviour beyond encode/decode, and data-page padding is written as zeros"
    requirement: "WIRE-06"
    verification: []
    human_judgment: true
    rationale: "Both are judgment prohibitions (plan frontmatter verification: judgment). Zero padding is additionally proven by aead_wire_messages_data_bodies_are_whole_pages, but 'no behaviour beyond encode/decode' has no executable assertion."

# Metrics
duration: 15min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 05: Message bodies — the base set, the new pairs, page-aligned data Summary

**`port::chan::msg` now encodes and decodes all 27 base 9P2000 bodies, the eight RFC section 6.2 new pairs plus the two re-key messages, and the D-09 parameter-then-zero-padded-4096-byte-page data bodies for `Twrite`, `Rread` and `Rstream`, pinned by ten hand-built wire tests.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-11-25T00:31:00Z
- **Completed:** 2026-11-25T00:46:00Z
- **Tasks:** 2 (two TDD pins, each RED → GREEN twice → pin advance)
- **Files modified:** 3 (1 created, 2 filled in)

## Accomplishments

- `port/src/chan/msg.rs` defines `Msg` over the base set 100-127 without 106 (D-28), `opcode`, `encode_body` and `decode_body` with a private checked cursor that requires exact consumption and returns only static `"chan: ..."` errors. Every length goes through `u16::try_from`/`u32::try_from`; no `as` cast narrows a length (T-04-14).
- Base bodies are the `Harvey` `convS2M` bodies byte for byte, with the test vectors written test-side from `fcall.h` and `convS2M.c` (oracle digests in the ledger). Strings are raw bytes; `Rstat`/`Twstat` stat blobs share the 16-bit length and a 65536-byte value is an encoder error, never truncated.
- The new pairs 130-137 use the RFC section 6.2 field order minus the envelope `size[4] type tag[2]`; `Trekey`/`Rrekey` 138/139 carry the 37-byte `kind[1] generation[4] confirm[32]` body with kinds 0/1/2 accepted and 3 rejected (P3). The module doc states they are encode and decode only (Phase 6 owns their semantics).
- The three data messages are the D-09 two-part body: parameters (`Twrite` `fid/offset/count`, `Rread` `count`, `Rstream` `stream_id/chunk_seq/count`) then data zero-padded to whole 4096-byte pages, the true count in the parameters (T-04-15). Decode enforces `data_len % PAGE_LEN == 0`, `count <= data_len < count + PAGE_LEN` and zero padding (T-04-16). `data_layout` exposes the 16/4/16-byte parameter split and page count for the transport (P5) and Phase 5.

## Task Commits

Each TDD step was committed atomically:

1. **Task 1 RED: base 9P2000 message set** - `ffc8345` (test)
2. **Task 1 GREEN: base 9P2000 message bodies** - `bb835c8` (feat)
3. **Task 1 pin advance: wire-messages-base** - `694cea0` (docs)
4. **Task 1 cleanup: checked walk-count conversions** - `530bd78` (refactor)
5. **Task 2 RED: new pairs and page-aligned data** - `204031f` (test)
6. **Task 2 GREEN: new pairs and the two-part data body** - `ea06e4b` (feat)
7. **Task 2 pin advance: wire-messages-data** - `4b52464` (docs)

## Files Created/Modified

- `port/src/chan/msg.rs` — `Msg` (27 base + 8 new-pair + `Trekey`/`Rrekey`), `opcode`, `encode_body`, `decode_body`, `DataLayout`, `data_layout`, `MAXWELEM`/`MAX_PARAMS_LEN`/`REKEY_*`, the checked `Cur`, and the little-endian writers.
- `port/tests/aead_wire.rs` — appended the base-set, string, walk, truncation, new-pair, re-key, data-page, data-bound and data-layout pins with hand-built byte helpers; the existing nine header/registry pins are untouched.
- `evidence/PINS-04-05.md` — the `ORACLE harvey …` digest line and `wire-messages-base` / `wire-messages-data` RED/GREEN-1/GREEN-2/ADVANCED (8 lines).

## Decisions Made

- **Hand-written `PartialEq`/`Eq` for `Msg`.** `crate::dat::Qid` derives only `Clone, Copy, Debug` and `dat.rs` must stay unchanged by this plan, so `Msg` derives `Clone, Debug` and compares `Qid` fields through `qid_eq`.
- **CamelCase variant identifiers.** `TleaseRevoke`/`RleaseAck` keep the Rust naming lint clean while `wire::opcode_name` returns the RFC spellings `Tlease_revoke`/`Rlease_ack`; the registry name strings are unchanged.
- **Pin advances are separate docs commits.** The plan's action listed only test+feat commits, but D-03 and the 04-04 ledger form require a committed pin advance, so each pin ends with a `docs(04-05): advance …` commit carrying its `ADVANCED` line.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `Msg` cannot derive `PartialEq`/`Eq` as specified**
- **Found during:** Task 1 (before the RED commit)
- **Issue:** The plan's action says `Msg` derives `PartialEq, Eq`, but `crate::dat::Qid` does not derive `PartialEq` and the acceptance criterion forbids editing `port/src/dat.rs`.
- **Fix:** `Msg` derives `Clone, Debug` and implements `PartialEq`/`Eq` by hand, comparing `Qid` fields through a private `qid_eq`. Behaviour is identical to a derive.
- **Files modified:** `port/src/chan/msg.rs`, `port/tests/aead_wire.rs` (a `qids_equal` helper for the walk-order assertion)
- **Verification:** all round-trip and walk-order pins pass; `cargo xtask test` green twice
- **Committed in:** `ffc8345` (RED), `bb835c8` (GREEN)

**2. [Rule 1 - Bug] Two `as u16` length casts in `encode_body`**
- **Found during:** Task 1 acceptance review, after GREEN
- **Issue:** The `Twalk`/`Rwalk` element counts were written with `wname.len() as u16`, which the task's acceptance criterion forbids even though `MAXWELEM` bounds the value.
- **Fix:** Replaced both with `u16::try_from(..).map_err(..)`; no length in `encode_body` is narrowed by a cast.
- **Files modified:** `port/src/chan/msg.rs`
- **Verification:** focused pins and `cargo xtask test` green after the change
- **Committed in:** `530bd78`

---

**Total deviations:** 2 auto-fixed (1 blocking/API, 1 bug)
**Impact on plan:** Neither changes the wire bytes or any public signature; the first is forced by the read-only `dat.rs` constraint, the second enforces the plan's own no-narrowing rule.

## Issues Encountered

- During Task 2 GREEN a mechanical rename of the enum variants to CamelCase also rewrote the two RFC opcode-name string literals in the test registry table, so `aead_wire_registry_table_is_pinned` failed in the first full-suite run. Restored the RFC spellings (`"Tlease_revoke"`, `"Rlease_ack"`); both subsequent `cargo xtask test` runs and the focused pins are green. The slip was caught before the GREEN commits and left no trace in history beyond the ledger note.

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None. `msg.rs` is fully implemented; the ten new variants carry encode and decode only by design (Phase 6 semantics), which the module doc states and the plan's transparency prohibition requires.

## Threat Flags

None beyond the plan's register: T-04-14 is mitigated by the checked `Cur` and the count/remaining comparisons before any copy, T-04-15 by zero-padded encode plus the decoder's zero-padding check, and T-04-16 by `count <= data_len < count + PAGE_LEN`.

## Next Phase Readiness

- `Verified::body()` can now be handed straight to `msg::decode_body(header.typ, body)`; `data_layout` gives the transport the parameter/page split for keystream placement (P5) and Phase 5 the DMA ingress split.
- Plan 04-06 (TLVs) shares no file with this plan; plan 04-13 can copy the 130-139 registry names and the D-09 body rule into the RFC amendment.
- `cargo xtask test` (twice), `check` and `clippy` exit 0 on the final tree; ten `aead_wire_messages_` tests pass alongside the nine header/registry pins and two tracer tests.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- `port/src/chan/msg.rs`, `port/tests/aead_wire.rs` and `evidence/PINS-04-05.md` exist on disk.
- All seven plan commits are ancestors of HEAD: `ffc8345`, `bb835c8`, `694cea0`, `530bd78`, `204031f`, `ea06e4b`, `4b52464`.
- The ledger carries one `ORACLE harvey …` line and eight `PIN …` lines.
- `port/src/devmnt.rs` and `port/src/dat.rs` are unchanged across the plan range.
- `cargo xtask test` (twice), `cargo xtask check` and `cargo xtask clippy` exit 0; ten `aead_wire_messages_` tests pass.
