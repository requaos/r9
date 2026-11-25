---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 18
subsystem: transport
tags: [rekey, zeroization, chacha20, send-half, deadline, tracer]

requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
    provides: the in-band re-key state machine, Session/SendHalf, seal_packet/open_packet (plans 04-07, 04-08, 04-16)
provides:
  - "Session::exchange_secret_zeroized and a complete Rekey::fail scrub (CR-01)"
  - "handle_ack idle reset, an overlap-free rekey_due and a second re-key per connection (WR-01)"
  - "SendHalf::send honours the acknowledgement deadline inside the send lock (WR-02)"
  - "seal_packet and open_packet through crypt_body, one P5 keystream layout (WR-03)"
  - "four mid-exchange/cutover fixture helpers reused by plans 04-19 to 04-22"
affects: [04-21, 04-23, 04-24, 04-25]

actuals:
  tokens: 10072
  tasks: 3
  commits: 9
  plan_head_before: 3041ded324249882e0dfd4af33da718711bd6eb1
  plan_head_after: 54848d48fb56e9ebc0eea365531ded3c606549eb

tech-stack:
  added: []
  patterns:
    - "Zeroization queries computed from storage, never from a flag (existing pattern extended)"
    - "Deadline checks inside the per-direction send lock, before sealing (D-21)"
    - "A session finishes zeroization the shared send half cannot reach (self-closed half)"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-18.md
  modified:
    - port/src/chan/transport.rs
    - port/tests/aead_transport.rs

key-decisions:
  - "Rekey::fail drops the exchange secret (Key is Zeroizing), zero-fills and clears out, and drops the transcript; keys_zeroized and exchange_secret_zeroized read that storage"
  - "handle_ack resets ready/own_marker_sent/request_sent/active/phase; rekey_due also requires both pending slots empty and no outstanding ack deadline"
  - "server_exchange_incoming accepts a Tauth only from Idle with both pending slots empty; any other Tauth closes through fail()"
  - "SendHalf::send checks state.deadline against the caller's now inside the send lock; seal/open/poll/rekey_step call self.fail() on a self-closed half"
  - "seal_packet/open_packet route the body through crypt_body so one P5 layout sits behind every public sealing function"

requirements-completed: [WIRE-03, WIRE-04, WIRE-05, WIRE-06]

coverage:
  - id: D1
    description: "CR-01: a failure or teardown mid-exchange leaves no live exchange secret or seam output; both queries answer true"
    requirement: WIRE-03
    verification:
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_failure_mid_exchange_zeroizes_exchange_secret"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_teardown_mid_exchange_zeroizes_exchange_secret"
        status: pass
    human_judgment: false
  - id: D2
    description: "WR-01: the re-key returns to idle and a second threshold crossing starts a second exchange; a server asks again"
    requirement: WIRE-04
    verification:
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_second_threshold_starts_a_second_exchange"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_server_requests_again_after_first_rekey"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_rekey_exchange_start_with_pending_keys_closes"
        status: pass
    human_judgment: false
  - id: D3
    description: "WR-02: the shared SendHalf closes on a withheld acknowledgement at the deadline and the session completes zeroization"
    requirement: WIRE-04
    verification:
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_send_order_ack_deadline_closes"
        status: pass
    human_judgment: false
  - id: D4
    description: "WR-03: seal_packet and open_packet use the P5 placement; a data page decrypts alone at block counter 1"
    requirement: WIRE-06
    verification:
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_tracer_data_body_uses_session_keystream_placement"
        status: pass
      - kind: unit
        ref: "port/tests/aead_transport.rs#aead_transport_tracer_non_data_body_is_one_stream_from_counter_zero"
        status: pass
    human_judgment: false
  - id: D5
    description: "IN-01 corrected only inside transport.rs: the module doc and Session::fail describe the shipped re-key state"
    verification:
      - kind: other
        ref: "grep -c 'Re-keying is plan 04-08' port/src/chan/transport.rs => 0"
        status: pass
    human_judgment: false

duration: 5 min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 18: gap-closure re-key lifecycle Summary

**Exchange-secret scrub, an idle re-key that runs again, a send-half acknowledgement deadline, and one P5 keystream behind the public helpers**

## Performance

- **Duration:** 5 min
- **Started:** 2026-11-25T04:41:11Z
- **Completed:** 2026-11-25T04:46:44Z
- **Tasks:** 3
- **Files modified:** 3 (two source, one evidence ledger)

## Accomplishments
- CR-01 closed: `Rekey::fail` drops the exchange secret, zero-fills and clears the seam output and drops the transcript; `Session::exchange_secret_zeroized` and `Rekey::keys_zeroized` answer from that storage, and a failure or `teardown()` mid-exchange leaves nothing live.
- WR-01 closed: `handle_ack` returns the end's own direction to idle; `rekey_due` also requires both pending slots empty and no outstanding deadline; a `Tauth` with pending keys held closes. A second threshold crossing starts a second exchange and a server asks a second time.
- WR-02 closed: `SendHalf::send` checks the stored deadline against the caller's `now` inside the send lock and writes nothing at the deadline; `seal`, `open`, `poll` and `rekey_step` complete zeroization of a self-closed half.
- WR-03 closed: `seal_packet` and `open_packet` route the body through `crypt_body`, so parameters sit at block counter 0 and data page `i` at `1 + 64 * i` behind every public sealing function.
- Tracer proven end-to-end: the CR-01 pin shows `ok` twice in `cargo xtask test-9p`, on the default and scalar-backend `port` leg.

## Task Commits

Each task was committed atomically (RED → GREEN, with the pin advance):

1. **Task 1 RED: exchange-secret pin** - `039f80e` (test)
2. **Task 1 GREEN: scrub on every failure path (CR-01)** - `a5dca03` (fix)
3. **Task 1 pin advance** - `073605f` (docs)
4. **Task 2 RED: second re-key and send-half deadline pins** - `087a4da` (test)
5. **Task 2 GREEN: idle re-key and deadline (WR-01, WR-02)** - `6b8b043` (fix)
6. **Task 2 pin advance** - `a741806` (docs)
7. **Task 3 RED: helper keystream pin** - `1f7e516` (test)
8. **Task 3 GREEN: helpers through crypt_body (WR-03)** - `6530fca` (fix)
9. **Task 3 pin advance** - `54848d4` (docs)

**Plan metadata:** recorded separately (SUMMARY + evidence commit).

## Files Created/Modified
- `port/src/chan/transport.rs` - the four fixes plus the `exchange_secret_zeroized` query; two stale IN-01 doc sentences corrected inside this file only.
- `port/tests/aead_transport.rs` - eight new pins and four fixture helpers.
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-18.md` - the RED/GREEN ledger for the three pins.

## Pin ledger (ADVANCED commits)
- `gap-cr01-exchange-secret` → **a5dca03**
- `gap-wr01-wr02-rekey-lifecycle` → **6b8b043**
- `gap-wr03-helper-keystream` → **6530fca**

## Fixture helpers added (recorded for plan 04-21)
All four live in `port/tests/aead_transport.rs`; `port/tests/common/aead_link.rs` is unchanged.
- `drive_exchange_to_client_twrite` — stops after the client's `Twrite` (client holds the exchange secret between `Done` and `finish_exchange`).
- `drive_exchange_to_server_rwrite` — stops after the server's `Rwrite` (server holds the secret and the seam output).
- `client_cutover` — one client-to-server marker/acknowledgement D-20 cutover.
- `server_cutover` — one server-to-client marker/acknowledgement D-20 cutover.

## Decisions Made
- The exchange secret is dropped (`None`) rather than overwritten, relying on `Key`'s `Zeroizing` drop; `out` is zero-filled and cleared before the pending/superseded loops.
- The idle reset clears `request_sent` and `own_marker_sent`; `start()` already clears `request_sent` when the requested exchange begins.
- `rekey_due` gained the two pending-slot conditions and the deadline condition, which also prevents a new exchange overlapping the previous generation.
- `SendHalf::send` checks `state.closed` and the deadline before sealing, inside the one lock scope (D-21).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Test 3 is a GREEN guard, not a RED pin**
- **Found during:** Task 2 RED
- **Issue:** The plan said all four Task 2 tests fail at RED, and named "the pending-keys `Tauth` is absorbed" as the failure. On the code as Task 1 left it the server is in `ExPhase::Done` after a cutover, and a `Tauth` outside `Idle` already closes, so the pin passes at RED. There is no reachable `Idle`-with-pending state before the WR-01 idle reset.
- **Fix:** Kept the pin as written — it exercises the GREEN-reachable state (the idle reset leaves `phase == Idle` with a pending slot held) and guards the new `pending` check in the `Tauth` arm. Recorded explicitly as a guard in `PINS-04-18.md` so the RED/GREEN story is not overstated.
- **Verification:** At RED `16 passed; 2 failed` for the `rekey` filter and `3 passed; 1 failed` for `send_order`; at GREEN `18` and `4` pass respectively.
- **Committed in:** `087a4da` (RED pin and ledger note)

### Additional commits
Three `docs(04-18)` pin-advance commits were made separately so each ledger advance names the fix commit's own hash (the PINS-04-08 form). They are evidence bookkeeping, not code changes.

---

**Total deviations:** 1 test-design deviation (documented, no code impact). **Impact:** none on the shipped behavior; the pin still guards the GREEN state.

## Issues Encountered
- The shared `SendHalf` accessor is on `Session`, so the Task 2 pin calls `client.session.send_half()`; two earlier in-file uses were left unchanged.

## User Setup Required
None - no external service configuration required.

## Threat Surface Scan
No new network endpoints, auth paths, file-access patterns or schema changes. The four threat-register mitigations this plan owns are implemented and pinned:
- T-04-54 (exchange secret / seam output) — `Rekey::fail`, both queries, the two mid-exchange pins.
- T-04-55 (one-shot re-key DoS) — idle reset in `handle_ack`, the second-exchange pins.
- T-04-56 (withheld ack on `SendHalf`) — deadline inside the send lock, `aead_transport_send_order_ack_deadline_closes`.
- T-04-57 (two keystream layouts) — both helpers through `crypt_body`.

## Next Phase Readiness
- CR-01, WR-01, WR-02 and WR-03 have a pin that was RED against the old code (or a documented guard) and is GREEN now.
- `nix develop -c cargo xtask test` (37 `ok` lines, 0 failures), `check` and `clippy` exit 0; `cargo xtask test-9p` exits 0 with the CR-01 pin twice.
- `port/src/chan.rs` is unchanged; no wire constant changed value; IN-01's other paragraphs in `chan.rs` and `lib.rs` and IN-02 to IN-07 stay open for the owner.
- Plans 04-19 to 04-22 can reuse `drive_exchange_to_client_twrite`, `drive_exchange_to_server_rwrite`, `client_cutover` and `server_cutover`.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- All four target paths exist.
- All nine plan commits are ancestors of HEAD.
- `port/src/chan.rs` is unchanged by this plan.
- The stale IN-01 sentence count inside `transport.rs` is 0.
