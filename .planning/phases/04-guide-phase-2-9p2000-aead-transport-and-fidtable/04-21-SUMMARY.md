---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 21
subsystem: transport
tags: [rekey, notag, afid, transcript, unpadded, routing, blake3, 9p2000]

requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
    provides: the in-band re-key state machine, Session/SendHalf and the P2/P3/P4 wire decisions (plans 04-07, 04-08, 04-13, 04-18)
provides:
  - "DECISION P9: the reserved exchange tag NOTAG (0xFFFF) and afid 0xFFFFFFFE, with the owner's Tstream/Rstream amendment and the second amendment covering Tcompound/Rcompound"
  - "transport::RekeyIncoming, transport::is_rekey_traffic and the tag-carrying Session::rekey_step (CR-02)"
  - "NOTAG on every Trekey/Rrekey and every in-band exchange message (WR-07, P3)"
  - "transport::exchange_frame: the unpadded standard 9P2000 frame the re-key transcript absorbs (CR-03, P2)"
  - "the end-to-end fixture routes every opened message through is_rekey_traffic before serving the tree"
affects: [04-22, 04-23, 04-24, 04-25]

actuals:
  tokens: 21829
  tasks: 3
  commits: 8
  plan_head_before: 5fa066201025162cfb610c8fcaa28b468b1baf20
  plan_head_after: ed0065c03def62cff98f9eb8a179c93ceb1b4ab3

tech-stack:
  added: []
  patterns:
    - "A shared connection carries two kinds of traffic; the reserved tag (with the afid on T-messages) is the router key, never the message type alone"
    - "One function builds every frame the transcript absorbs, so the hashed form cannot drift from the wire form"
    - "A reserved-tag message that does not fit the route closes; any other tag is ordinary traffic and is a no-op"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-21.md
  modified:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-WIRE-DECISIONS.md
    - port/src/chan/transport.rs
    - port/tests/common/aead_link.rs
    - port/tests/aead_transport.rs
    - port/tests/aead_negotiate.rs
    - .planning/rfc-9p2000-aead.md
    - .planning/PROJECT.md

key-decisions:
  - "The exchange is self-identifying: tag NOTAG (0xFFFF) on all six exchange messages and on Trekey/Rrekey, afid 0xFFFFFFFE on Tauth, Twrite and Tread; Rwrite, Rread and Rauth carry no fid and are identified by tag plus type"
  - "The owner's P9 amendment: a Tstream is exchange traffic only under NOTAG with the afid as its 8-byte FidHandle (afid little-endian in the low four bytes, high four zero); an Rstream under NOTAG (it carries no fid); with any other tag both are ordinary traffic"
  - "The owner's second P9 amendment: Tcompound and Rcompound are exchange-space only under NOTAG; neither carries a top-level fid, so NOTAG plus the type alone identifies them; the exchange dispatches neither today, so a NOTAG compound fails closed (fail() + Err(Closed)) and is never delivered as ordinary traffic"
  - "is_rekey_traffic(tag, message) is the single router predicate; rekey_step treats a reserved-tag message that fails it as a protocol violation (close) and any other tag as ordinary traffic (no packet, no state change)"
  - "exchange_frame builds the absorbed standard frame with the true count and exactly that many data bytes; the sealed body stays the P4 page-padded body, so the wire size of an exchange packet is unchanged"
  - "The fixture keeps rekey(exchange, Option<&Msg>) as a NOTAG-forwarding convenience and adds rekey_tagged for explicit tags; receive helpers gained recv_tagged"

patterns-established:
  - "Reserved-tag routing: (tag, type, afid) decides the route; type alone never does"
  - "One unpadded frame builder for every absorbed re-key transcript message (P2)"

requirements-completed: [WIRE-04]

coverage:
  - id: D1
    description: "CR-02: an ordinary request or reply crossing an in-band exchange is delivered unchanged and the exchange continues to generation 1"
    requirement: WIRE-04
    verification:
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_ordinary_read_during_exchange_is_delivered"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_ordinary_write_at_server_is_not_consumed"
        status: pass
      - kind: integration
        ref: "port/tests/aead_negotiate.rs#aead_negotiate_end_to_end_read_during_rekey_is_served"
        status: pass
    human_judgment: false
  - id: D2
    description: "WR-07 and P9 routing: the control and exchange messages carry NOTAG, the afid, and a reserved-tag violation closes the receiver"
    requirement: WIRE-04
    verification:
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_control_messages_carry_notag"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_exchange_messages_carry_reserved_tag_and_afid"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_exchange_tag_with_wrong_fid_closes"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_control_with_wrong_tag_closes"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_stream_notag_afid_is_exchange"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_stream_ordinary_is_not_consumed"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_compound_ordinary_is_not_consumed"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_compound_notag_closes"
        status: pass
    human_judgment: false
  - id: D3
    description: "CR-03: the re-key transcript absorbs the unpadded standard frames; a 21-byte Twrite is 44 bytes and an empty one is 23"
    requirement: WIRE-04
    verification:
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_transcript_twrite_frame_is_44_bytes_unpadded"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_transcript_empty_twrite_frame_is_23_bytes"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_wire_keys_and_confirm_equal_test_side_derivation"
        status: pass
    human_judgment: false
  - id: D4
    description: "The RFC, PROJECT.md and the wire-decision ledger state the values the code sends (P2, P3 and P9)"
    verification: []
    human_judgment: true
    rationale: "Whether prose and code agree is a reading judgment; the automated greps only prove the notes exist and quote the superseded wording."
  - id: D5
    description: "The owner is told the P4 limit the exchange still carries and the wave-3 worktree setting"
    verification: []
    human_judgment: true
    rationale: "An owner-facing instruction; the owner must act on it before wave 3."

duration: 7 min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 21: the re-key on the wire Summary

**The in-band exchange became self-identifying by reserved tag `NOTAG` and afid `0xFFFFFFFE`, so ordinary traffic crosses a re-key untouched, and the re-key transcript now hashes the unpadded standard frames P2 pins**

## Performance

- **Duration:** 7 min (commit span)
- **Started:** 2026-11-25T13:18:37Z (first plan commit)
- **Completed:** 2026-11-25T13:25:35Z (last plan commit before the SUMMARY)
- **Tasks:** 3
- **Files modified:** 7 (three source/test, one fixture, one decision ledger, two documents)

## The owner's decision, quoted

> DECISION P9: amended (2026-11-25) — accepted as proposed (tag NOTAG 0xFFFF on all six exchange messages, afid 0xFFFFFFFE), with the owner's addition that Tstream, Rstream, Tcompound and Rcompound are covered by the same reserved routing: a sealed Tstream or Rstream is exchange traffic only when its tag is NOTAG, and for Tstream its FidHandle names the exchange afid 0xFFFFFFFE; Rstream carries no fid, so NOTAG plus its type identifies it; Tcompound and Rcompound carry no top-level fid, so NOTAG plus the type alone identifies them as exchange-space; any message in this family with any other tag is ordinary traffic and the re-key state machine never consumes it, and a NOTAG message the exchange does not expect closes the connection fail-closed rather than being delivered as ordinary traffic. If the exchange ever carries one of these types, it enters the P2 transcript in the same standard 9P2000 body form with the data unpadded. P4 is unchanged.

Both amendments are implemented in `is_rekey_traffic`, and the safe failure is in `rekey_handle`. A `Tstream` is exchange traffic only when the tag is `NOTAG` and its 8-byte `FidHandle` is `0xFFFFFFFE` little-endian in the low four bytes with the high four zero; an `Rstream` is exchange traffic when the tag is `NOTAG`. A `Tcompound` or `Rcompound` is exchange-space when the tag is `NOTAG` — neither carries a top-level fid, so no afid comparison applies — and ordinary traffic under any other tag. The exchange never sends any of these four types: this is routing and recognition only. Because it dispatches neither compound, `rekey_handle` now has an explicit `Tcompound`/`Rcompound` arm that fails closed (`fail()` and `Err(Closed)`, which zeroizes) on a `NOTAG` compound rather than letting the catch-all ignore it or delivering it as ordinary traffic. The value in the `DECISION` line is the value in the code and its pins.

## Second amendment (2026-11-25): Tcompound/Rcompound

The owner added `Tcompound`/`Rcompound` to the same reserved routing after the first amendment shipped. What changed:

- `04-WIRE-DECISIONS.md`: the single `DECISION P9:` line now carries both amendments; `PROPOSAL P9:` is untouched.
- `port/src/chan/transport.rs`: `is_rekey_traffic` classifies a `NOTAG` `Tcompound`/`Rcompound` as exchange traffic (no fid, so the tag alone decides), and `rekey_handle` gained an explicit compound arm that calls `fail()` and returns `Err(Closed)` for a `NOTAG` compound; an ordinary-tag compound is an `Ok(())` no-op.
- `port/tests/aead_transport.rs`: `aead_transport_rekey_compound_ordinary_is_not_consumed` (ordinary `Tcompound` and `Rcompound` are delivered unchanged and do not close) and `aead_transport_rekey_compound_notag_closes` (a `NOTAG` `Tcompound`/`Rcompound` classifies as exchange traffic, closes the session and zeroizes the keys).
- `.planning/rfc-9p2000-aead.md` and `.planning/PROJECT.md`: both exchange-family enumerations now name `Tcompound`/`Rcompound`, with the same "`NOTAG` plus the type alone identifies them" wording and the fail-closed rule.

New pins: `aead_transport_rekey_compound_ordinary_is_not_consumed`, `aead_transport_rekey_compound_notag_closes`. Verification after the change: `nix develop -c cargo xtask test` twice, `cargo xtask check`, `cargo xtask clippy` and `cargo xtask test-9p` all exit 0. No other wire value changed.

## Accomplishments

- **CR-02 closed.** `is_rekey_traffic(tag, message)` is the router: true only under `NOTAG`, and for `Tauth`/`Twrite`/`Tread` only when the body names afid `0xFFFFFFFE`. `rekey_step` gives ordinary traffic back as a no-op (no packet, no state change) and closes on a reserved-tag message that does not fit the route. A live mount's `Tread`/`Rread`/`Twrite` crossing an exchange is delivered unchanged; the exchange still reaches generation 1.
- **WR-07 closed.** `REKEY_TAG` and `REKEY_EXCHANGE_TAG` are `crate::devmnt::NOTAG`; a `Trekey`/`Rrekey` with any other tag closes through `fail()`, and the marker, ACK and REQUEST header tag bytes are pinned.
- **CR-03 closed.** `exchange_frame` builds `size[4] type[1] tag[2]` then the standard body, with `Twrite`, `Rread` and `Rstream` writing their true count and exactly that many data bytes. A 21-byte `Twrite` is absorbed as exactly 44 bytes; an empty one as 23. The sealed exchange body stays the P4 page-padded body, so only what is hashed changed.
- **Documents follow the code.** Two dated gap-closure notes in RFC sections 3.2 and 6.1 (the second quotes and extends the section 4.2 Tag sentence) and one dated `PROJECT.md` Key Decisions row cite `DECISION P9` and the pins.

## Task Commits

Each task was committed atomically (RED -> GREEN -> pin advance, D-03):

1. **Task 1: owner decision on P9** - `9ea797c` (docs)
2. **Task 2 RED: exchange-routing and control-tag pins** - `4bb286a` (test)
3. **Task 2 GREEN: route by the P9 tag and afid (CR-02, WR-07)** - `bb7e83c` (fix)
4. **Task 2 pin advance** - `11d8c83` (docs)
5. **Task 3 RED: unpadded-transcript pins** - `eaf1b20` (test)
6. **Task 3 GREEN: the transcript absorbs the unpadded frames (CR-03, P2)** - `62f14e7` (fix)
7. **Task 3 documents: RFC and PROJECT.md** - `c2c54f1` (docs)
8. **Task 3 pin advance** - `ed0065c` (docs)

**Plan metadata:** committed separately (SUMMARY.md only; STATE.md and ROADMAP.md are the orchestrator's, per this worktree dispatch).

## Pin ledger (ADVANCED commits)

- `gap-cr02-wr07-routing` -> **bb7e83c** (RED `4bb286a`: 7 routing/tag pins and all five `aead_negotiate_end_to_end_*` pins failed; the latter fail because the RED stub ignores the tag and consumes an ordinary `Tread`, which is exactly CR-02)
- `gap-cr03-unpadded-transcript` -> **62f14e7** (RED `eaf1b20`: the 44-byte, the 23-byte and the independent pin failed on MAC assertions)

## Files Created/Modified

- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-WIRE-DECISIONS.md` - `PROPOSAL P9` and the owner's `DECISION P9: amended`, plus the header corrected from eight to nine proposals.
- `port/src/chan/transport.rs` - `RekeyIncoming`, `is_rekey_traffic` (including the `Tstream`/`Rstream` and `Tcompound`/`Rcompound` arms), the tag-carrying `rekey_step`/`rekey_handle` (and its fail-closed compound arm), `exchange_frame`, the reserved constants and afid checks on the `Tauth`/`Twrite`/`Tread` arms.
- `port/tests/common/aead_link.rs` - `recv_tagged`, `rekey_tagged`, and `E2eServer::handle_one` routing every opened message through `is_rekey_traffic`.
- `port/tests/aead_transport.rs` - ten routing/control/stream/compound pins, two unpadded-transcript pins, the hand-written frame helpers and the rewritten `test_generations`.
- `port/tests/aead_negotiate.rs` - `aead_negotiate_end_to_end_read_during_rekey_is_served` and the routed `handle_one` call sites.
- `.planning/rfc-9p2000-aead.md` - two dated gap-closure notes (sections 3.2 and 6.1).
- `.planning/PROJECT.md` - one dated Key Decisions row for P9 and the footer date.
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-21.md` - eight `PIN` lines across the two pins.

## Deviations from Plan

### Auto-fixed Issues

None. The two test-behaviour adjustments below are recorded as deliberate design notes, not bug fixes.

**1. [Test design] The fixture keeps a `NOTAG`-forwarding `rekey` convenience**
- **Found during:** Task 2 RED
- **Issue:** the plan said to move every caller of the fixture's four `rekey` helpers to the new signature. Moving all 77 call sites would have forced a fake tag variable into tests that never needed one.
- **Fix:** `rekey(exchange, Option<&Msg>)` forwards `NOTAG` and `rekey_tagged(exchange, Option<(u16, &Msg)>)` passes any tag; `recv_tagged` returns the header tag. Only the pins that test routing use the tagged form.
- **Committed in:** `4bb286a`

**2. [Test design] Three pin-advance commits beyond the plan's named commits**
- **Found during:** Tasks 2 and 3
- **Issue:** a pin's `ADVANCED` line must name the fix commit's hash, which cannot be known inside that commit.
- **Fix:** one `docs(04-21): advance pin ...` commit per pin, the PINS-04-08/04-18 form. Evidence bookkeeping only; no code change.
- **Committed in:** `11d8c83`, `ed0065c`

**3. [RED signal] `aead_transport_rekey_stream_ordinary_is_not_consumed` passes at RED**
- The code already ignored `Tstream`/`Rstream` through `rekey_handle`'s catch-all, so this pin guards the owner's amendment rather than changing it. Recorded in `PINS-04-21.md` so the RED/GREEN story is not overstated.

---

**Total deviations:** 0 auto-fixed code issues; 3 recorded test/evidence-design notes. **Impact:** none on the shipped behaviour.

## Issues Encountered

- The RED `is_rekey_traffic` stub (type-only, the code's rule before this plan) routes an ordinary `Tread` into the exchange state machine, so the five `aead_negotiate_end_to_end_*` pins failed at RED. That is CR-02 itself; the GREEN router is what makes them pass again, and `PINS-04-21.md` records it.
- `Rauth` briefly fell outside `is_rekey_traffic` while the router was being written; the RED/GREEN pin run caught it before the GREEN commit. No committed state carried it.

## The P4 limit the exchange still carries (for the owner)

`DECISION P4` is unchanged and `DECISION P9` says so: the sealed exchange `Twrite` and `Rread` still carry whole 4096-byte pages on the wire. Measured on this code:

- the sealed exchange `Twrite` is 4172 bytes: the 28-byte header, the 4112-byte P4 body (16 parameter bytes plus one 4096-byte page) and the 32-byte MAC;
- the sealed exchange `Rread` is 4160 bytes: the 28-byte header, the 4100-byte P4 body (4 parameter bytes plus one page) and the 32-byte MAC.

So a session whose negotiated msize is below 4172 cannot re-key: `packet_len` refuses the packet, `seal_into` fails and the one `fail()` path closes the connection. Changing that would amend P4 and is not proposed here. What this plan changed is only what the transcript hashes: the same exchange is absorbed as 44 bytes for the `Twrite` (and 32 for a 21-byte `Rread`), not 4119.

## Threat Surface Scan

No new network endpoint, auth path, file-access pattern or schema. The plan's four mitigations are implemented and pinned:

- T-04-62 (an ordinary reply absorbed into the transcript): the router by tag and afid; `aead_transport_rekey_ordinary_read_during_exchange_is_delivered`, `aead_negotiate_end_to_end_read_during_rekey_is_served`.
- T-04-63 (ordinary traffic closing the connection): `rekey_step` returns `Ok` with no packet; `aead_transport_rekey_ordinary_write_at_server_is_not_consumed`.
- T-04-64 (a reserved tag on the wrong fid, or a control message on an ordinary tag): `aead_transport_rekey_exchange_tag_with_wrong_fid_closes`, `aead_transport_rekey_control_with_wrong_tag_closes`.
- T-04-65 (undocumented padded/typed inputs): `exchange_frame`, the hand-written 44-byte pin and the dated RFC notes checked against the constants.

## User Setup Required

None - no external service configuration required.

## Owner action before wave 3

Plans 04-23, 04-24 and 04-25 run only on the main working tree. Set `workflow.use_worktrees` to `false` in `.planning/config.json` (or through `/gsd-config`) before continuing `/gsd-execute-phase 4 --gaps-only`, and set it back to `true` after plan 04-25. The executor has not changed the setting.

## Next Phase Readiness

- CR-02, CR-03 and WR-07 each have a pin that was RED against the pre-plan code and is GREEN now; plan 04-13's "documents equal code" prohibition holds again for P2, P3 and P9.
- `nix develop -c cargo xtask test` ran twice (37 `test result: ok`, 0 failed); `check`, `clippy` and `test-9p` exit 0, and `test-9p` shows the 44-byte pin and the control-tag pin twice (default and scalar-backend legs).
- `port/src/chan/msg.rs` and `port/src/devmnt.rs` are unchanged; no wire constant in `msg.rs` changed. The four review findings outside the gaps (IN-02 to IN-04) stay open for the owner.
- Any change under `port/src` changes kernel bytes; plans 04-24 and 04-25 redo the byte evidence and the owner stop. This plan built no kernel.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- All eight plan commits are ancestors of HEAD.
- `04-WIRE-DECISIONS.md` has one `PROPOSAL P9:` line and one `DECISION P9: amended` line.
- `is_rekey_traffic` and `RekeyIncoming` are in `port/src/chan/transport.rs` (grep count 2).
- `cargo xtask test` twice, `check`, `clippy` and `test-9p` all exit 0; `aead_transport` has 53 passing tests, `aead_negotiate` 11, `devmnt_test` 18.
- The RFC has two `r9 Phase 4 gap closure` notes and two copies of the section 4.2 Tag sentence; `PROJECT.md` contains `DECISION P9`.
- The pin ledger has exactly eight `PIN` lines for the two pins.
- After the second amendment: `nix develop -c cargo xtask test` twice, `check`, `clippy` and `test-9p` all exit 0; `aead_transport` has 55 passing tests. `04-WIRE-DECISIONS.md` still has one `PROPOSAL P9:` and one `DECISION P9:` line; `PROPOSAL P9:` is unchanged.
