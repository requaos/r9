---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "10"
subsystem: kernel-transport-fidtable
tags: [9p2000-aead, tpht, fid-table, single-read-resolve, stateless-cap, blake3, roaring-bitmap, sweep, tdd]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "ds::tpht::flattened::{FlattenedTphtShard, with_banks_and_probe, deref_tiny}; ds::roaring::RoaringBitmap::{difference, remove, len}; the public ds::tpht::Probe seam"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-09)
    provides: "StatefulHandle/StatelessCap/FidEntry layouts, FID_LIVE, slot_id/read_slot and the arena slot cleared on remove"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-03)
    provides: "crypto::session::cap_tag/cap_verify — the 8-byte capability tag under a caller-supplied domain Key"
provides:
  - "port::fid::{FidShard, FidTable} with new/with_probe/insert/resolve/clunk/set_epoch/issue_cap/verify_cap/mark_reclaimed/sweep"
  - "single-read resolve: one arena read, zero header reads, identity compared in registers"
  - "stateless capabilities issued and verified with zero FID memory"
  - "the failover sweep as one RoaringBitmap difference per shard"
affects: [04-11 single-read resolve server, 04-16 sweep wiring, 04-17 test-9p, phase-05 page sealing, phase-06 domain-secret provisioning]

actuals:
  tokens: 11568
  tasks: 3
  commits: 9
  plan_head_before: b636e0b79a2952063e42c208e8ccb9d11f9ac9da
  plan_head_after: 084e8941cc62a32f2889cc86db0450abbac685a5

tech-stack:
  added: []
  patterns:
    - "Home-only TPHT shards: every entry has a TinyPtr, so HomeFull falls over to the next shard instead of spilling"
    - "A handle is a (fid_key, ptr, shard_id, epoch) tuple; resolve validates the whole entry after one deref_tiny read with register-only comparisons"
    - "Capabilities are bearer tokens signed under a caller-supplied domain Key in crypto::session; port names no hashing primitive and holds no secret material"
    - "The sweep is one bitmap difference (active \\ reclaimed); swept ids are removed from active one at a time so survivors need no second set operation"
    - "The test-side CountingProbe is defined against the public ds::tpht::Probe trait, since ds's probe is private to its test module"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-10.md
  modified:
    - port/src/fid.rs
    - port/tests/aead_fid.rs

key-decisions:
  - "FidShard wraps FlattenedTphtShard<8, P> with zero banks; FidTable boxes 1..=256 shards and falls over on HomeFull, recording the landing shard in the handle (RESEARCH Pitfall 3)"
  - "The home shard is a SplitMix64 mix of the (client_id << 32 | fid_key) key modulo the shard count; insert walks forward from it on HomeFull"
  - "resolve rejects an out-of-range shard_id before deref_tiny, then compares fid_key, client_id, entry/handle/table epoch and FID_LIVE — all register arithmetic after the one read"
  - "issue_cap signs cap.body() with crypto::session::cap_tag and verify_cap checks the epoch then defers the constant-time compare to cap_verify, so neither touches a shard or bitmap"
  - "sweep computes active \\ reclaimed with one difference per shard, removes each orphan from active as it is collected, and empties the reclaim window"
  - "Two small read-only accessors were added for the required pins: home_shard (deterministic placement) and bitmap_lens (observing that a capability touches no bitmap); both are documented as test/metrics helpers, not part of the hot path"

patterns-established:
  - "A pin's RED stub keeps the real signature with a wrong body, so every RED failure is a behavioural assertion (D-03, Pitfall 11)"
  - "A table pin proves a one-read path by resetting a shared counting probe and asserting the (arena, header) pair"

requirements-completed: [WIRE-07, WIRE-08, WIRE-09]

coverage:
  - id: D1
    description: "A handle resolves to its FidEntry in exactly one arena read and zero header reads"
    requirement: WIRE-08
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_resolve_is_one_arena_read"
        status: pass
      - kind: integration
        ref: "cargo xtask test (both GREEN runs, rc 0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A handle from another client or epoch, a clunked handle and an out-of-range shard id do not resolve; the out-of-range case reads nothing"
    requirement: WIRE-08
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_resolve_rejects_wrong_client_epoch_and_shard"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_resolve_clunked_handle_does_not_resolve"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_resolve_two_clients_same_fid_number"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_resolve_falls_over_to_the_next_shard_when_home_is_full"
        status: pass
    human_judgment: false
  - id: D3
    description: "A capability is issued and verified with zero FID memory; two tables with one domain key verify each other and a different key rejects"
    requirement: WIRE-08
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_stateless_uses_zero_fid_memory"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_stateless_two_tables_one_key_verify_each_other"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_stateless_different_key_rejects"
        status: pass
    human_judgment: false
  - id: D4
    description: "The 8-byte tag is the truncation of keyed BLAKE3 over the literal label and the 16-byte wire body; any flipped byte and a wrong epoch fail verification"
    requirement: WIRE-07
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_stateless_tag_is_keyed_blake3_of_label_and_wire_body"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_stateless_every_flipped_byte_rejects"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_stateless_wrong_epoch_rejects"
        status: pass
    human_judgment: false
  - id: D5
    description: "The sweep returns exactly active minus reclaimed on randomized inputs; reclaimed fids survive and swept fids are rejected; the reclaim window resets"
    requirement: WIRE-09
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_sweep_matches_set_model_on_random_inputs (64 seeds vs a BTreeSet model)"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_sweep_survivors_resolve_and_orphans_do_not"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_sweep_edges"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_sweep_resets_the_reclaim_window"
        status: pass
    human_judgment: false
  - id: D6
    description: "All three pins are recorded RED -> GREEN-1 -> GREEN-2 -> ADVANCED in the phase evidence"
    verification:
      - kind: other
        ref: "grep -cE '^PIN (fid-resolve|fid-stateless|fid-sweep) (RED|GREEN-1|GREEN-2|ADVANCED)' evidence/PINS-04-10.md == 12"
        status: pass
    human_judgment: false
  - id: D7
    description: "The module doc states a resolve is not a consistent snapshot and the capability is a bearer token with a 2^-64 forgery bound; client_id has no authenticated source yet"
    requirement: WIRE-08
    verification: []
    human_judgment: true
    rationale: "Prose guarantees in the module documentation (the torn-read tolerance, the bearer-token property and its forgery bound, and the Phase 6 client_id note) are reviewed by a person at the phase-end code review, not asserted by a test"

# Metrics
duration: "~30min"
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 10: `FidTable` and `FidShard` Summary

**`FidTable` on home-only TPHT shards: a handle resolves in one arena read and zero header reads, a `StatelessCap` is issued and verified under the domain secret with zero FID memory, and the failover sweep is one `RoaringBitmap` difference per shard.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-11-25T00:15:00Z (approx; before the first RED commit)
- **Completed:** 2026-11-25T00:45:00Z
- **Tasks:** 3
- **Files modified:** 3 (2 modified, 1 created)
- **Commits:** 9 measured (`git rev-list --count b636e0b..HEAD` before the SUMMARY commit)

## Accomplishments

- `port/src/fid.rs` gains `FidShard<P>` (a home-only `FlattenedTphtShard<8, P>` plus `active`/`reclaimed` `Lock<RoaringBitmap>` fields) and `FidTable<P>` (a boxed slice of 1..=256 shards, the caller-supplied `crypto::key::Key` domain secret and a `u16` epoch). WIRE-08.
- `resolve` rejects an out-of-range `shard_id` before any read, calls `deref_tiny` exactly once and compares `fid_key`, `client_id`, both epochs and `FID_LIVE` in registers: the counting probe asserts `(arena_reads, header_reads) == (1, 0)`. Wrong client, wrong epoch, a bumped table epoch and a clunked handle all fail. T-04-32, T-04-33, T-04-35.
- `issue_cap` fills the 24-byte capability at the table epoch and signs its first 16 bytes with `crypto::session::cap_tag`; `verify_cap` checks the epoch then defers the constant-time compare to `crypto::session::cap_verify`. Neither touches a shard or a bitmap, so two tables with one key verify each other and a different key rejects. WIRE-07, D-13, D-14, P8, T-04-34. The independent tag pin computes its expected tag with `crypto::mac::blake3_keyed` over a hand-written body and a literal label, never through the code under test.
- `mark_reclaimed` resolves the handle then records its dense slot id; `sweep` takes `active \ reclaimed` with one `difference` per shard, removes each orphan through the bounds-checked `read_slot`/`remove` path and resets the reclaim window. The model pin checks all 64 seeds against a `BTreeSet` of active minus reclaimed. WIRE-09.
- `port/tests/aead_fid.rs` grows from 9 layout pins to 24: 5 `aead_fid_resolve_*`, 6 `aead_fid_stateless_*` and 4 `aead_fid_sweep_*`, plus the test-side `CountingProbe` defined against the public `ds::tpht::Probe` trait.

## Task Commits

Each task was committed atomically (test -> feat -> ledger docs, the plan 04-09 pin precedent):

1. **Task 1 RED: single-read fid resolution pin** — `313517f` (test)
2. **Task 1 GREEN: FidTable/FidShard on TPHT, single-read resolve** — `ca29a14` (feat)
3. **Task 1 ledger: advance `fid-resolve`** — `e9a77fc` (docs)
4. **Task 2 RED: stateless capabilities pin** — `4d05e54` (test)
5. **Task 2 GREEN: `StatelessCap` issue and verify** — `298498b` (feat)
6. **Task 2 ledger: advance `fid-stateless`** — `06094cd` (docs)
7. **Task 3 RED: failover sweep pin** — `b551836` (test)
8. **Task 3 GREEN: `FidShard` failover sweep** — `ea7ef4d` (feat)
9. **Task 3 ledger: advance `fid-sweep`** — `084e894` (docs)

**Plan metadata:** the SUMMARY commit follows (docs).

## Files Created/Modified

- `port/src/fid.rs` — the table half of WIRE-07/08/09: `FidShard`, `FidTable`, `insert`, `resolve`, `clunk`, `set_epoch`, `issue_cap`, `verify_cap`, `mark_reclaimed`, `sweep`, `home_shard`, `bitmap_lens`, plus the module-doc sections on the table, the bearer-token property and the unauthenticated `client_id`.
- `port/tests/aead_fid.rs` — 15 new pins (5 resolve, 6 stateless, 4 sweep), the `CountingProbe`, `domain_key`, `entry_for`, `fixed_qid`, `XorShift64` and `handle_slot` helpers.
- `evidence/PINS-04-10.md` — the three pin records (`fid-resolve`, `fid-stateless`, `fid-sweep`), 12 `PIN` lines in total.

## Decisions Made

- **Home-only shards with fall-over.** A `FidTable` inserts into the key's home shard and walks forward on `HomeFull`, recording the landing shard in the handle; this keeps every entry a `TinyPtr` holder (RESEARCH Pitfall 3).
- **`resolve` checks the shard range first, then reads once.** The out-of-range rejection is before `deref_tiny`, so a hostile `shard_id` causes zero memory accesses (T-04-35, pinned with a zero-read probe).
- **The capability never touches table state.** The tag and key bytes stay in `crypto/`; `port` names only `cap_tag`/`cap_verify` and holds no hashing primitive, no `static` and no constant key material.
- **The sweep removes swept ids from `active` as it collects them.** That is one bitmap `difference` per shard (the one WIRE-09 names) and no second set operation for the survivors.

## Deviations from Plan

### Auto-fixed / added issues

**1. [Rule 2 - Missing Critical] Added two read-only accessors the required pins need**

- **Found during:** Task 1 (fall-over pin) and Task 2 (zero-FID-memory pin)
- **Issue:** the plan's tests require observability the artifact list did not name: the fall-over pin must know which shard a key homes to so it can fill one deterministically, and the stateless pin must assert both bitmaps of every shard stay empty. `FidTable`'s fields are private and the integration test cannot reach them.
- **Fix:** added `FidTable::home_shard(client_id, fid_key) -> u8` (documented placement query) and `FidTable::bitmap_lens(shard_id) -> Option<(u64, u64)>` (documented test/metrics accessor). Both are read-only and neither is on the hot path; `issue_cap`/`verify_cap` never call them.
- **Files modified:** `port/src/fid.rs`
- **Verification:** the fall-over pin fills shard 0 with 16 `home_shard == 0` keys and the zero-memory pin asserts `Some((0, 0))` on every shard; both pass.
- **Committed in:** `ca29a14` (Task 1 feat) — `bitmap_lens` is exercised from Task 2 onward.

### Process deviations

**2. Three commits per task, not two.** Each pin's `ADVANCED` line must name the GREEN commit, which cannot ride inside that commit, so a `docs` commit follows each GREEN. This is the plan 04-09 precedent: 9 commits, no extra code.

---

**Total deviations:** 1 auto-added (1 missing critical surface), 1 process note.
**Impact on plan:** The two accessors are read-only and documented as observability helpers; they add no behaviour to the resolve, capability or sweep paths. The extra docs commits carry only ledger lines.

## Issues Encountered

- **Duplicate insert after a fall-over is not detected across shards (latent, not reachable in the walk protocol).** The TPHT `insert` reports `Exists` only within the shard it is called on. If a fid fell over to shard 1 and shard 0 later frees a slot, a repeated insert of that fid could land in shard 0 and create a second entry. The server allocates each fid once and does not retry a failed insert, so the normal protocol never reaches this; it is recorded here for the phase-end code review rather than fixed (fixing it would cost a cross-shard read on every insert).
- **`client_id` has no authenticated source yet.** `insert` takes whatever the connection assigned; this phase has no authentication, so the test server assigns one per connection. Phase 6 owns the real source. Recorded in the module doc and below.

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None. Every public method has its real body; the RED stubs (`resolve` returning an error, a zero capability tag with `verify_cap` returning `Ok`, `sweep` returning an empty vector) were all replaced in the GREEN commits.

## Threat Flags

None beyond the plan's `<threat_model>`. The new surface is the same client→handle and holder→verify_cap boundaries T-04-32 to T-04-35 already register, and each is now covered by a pin.

## Next Phase Readiness

- **WIRE-07, WIRE-08 and WIRE-09 hold.** `resolve` is one arena read; the capability is zero FID memory; the sweep equals `active \ reclaimed`. Plan 04-11 (the single-read resolve server) and 04-16 (sweep wiring) consume these methods; 04-17's `test-9p` runs this file.
- **`client_id` is the test server's to assign.** Phase 6 defines the authenticated source; until then `insert` trusts its caller.
- **The capability is a bearer token within the domain and a failed verification is connection-fatal.** The transport (plan 04-11) must treat `verify_cap`'s `Err` as closing the connection (P8); the forgery bound is 2^-64 per attempt over an authenticated connection.
- **Provisioning the domain secret is Phase 6.** `FidTable::new` takes the `crypto::key::Key` as a parameter; nothing in `port/src` creates, defaults or stores one.
- **No `ds/` change in this plan.** `git diff` of `ds/` against the plan base is empty; the slot clearing, `slot_id` and `read_slot` landed in plan 04-09.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- All four files exist on disk (`port/src/fid.rs`, `port/tests/aead_fid.rs`, `evidence/PINS-04-10.md`, `04-10-SUMMARY.md`).
- All nine plan commits are ancestors of HEAD (`313517f`, `ca29a14`, `e9a77fc`, `4d05e54`, `298498b`, `06094cd`, `b551836`, `ea7ef4d`, `084e894`).
- `cargo test -p port --test aead_fid` reports 24 passed, 0 failed (9 layout/qid + 5 resolve + 6 stateless + 4 sweep).
- `cargo xtask test` x2, `cargo xtask check` and `cargo xtask clippy` all exit 0; clippy names neither `port/src/fid.rs` nor `port/tests/aead_fid.rs`.
- The pin ledger carries 12 `PIN (fid-resolve|fid-stateless|fid-sweep) (RED|GREEN-1|GREEN-2|ADVANCED)` lines.
- `git diff <plan base> HEAD -- ds/` is empty.

