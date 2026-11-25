---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 19
subsystem: wire-fid-table
tags: [fid, tpht, wire-08, wr-04, wr-06, wr-09, tdd, pin-ledger]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "ds::tpht::flattened::FlattenedTphtShard (deref_tiny, insert, lookup, remove), ds::tpht::HOME_SLOTS"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
    provides: "plans 04-09 and 04-10: the FidEntry layout, FidShard/FidTable, the one-read resolve and the sweep"
provides:
  - "FidEntry::matches — resolve's register comparisons as one named method"
  - "client id 0 reserved: insert and resolve refuse it; a cleared or torn identity word matches nobody"
  - "resolve refuses a ptr above ds::tpht::HOME_SLOTS before the arena read (the WR-06 fold)"
  - "insert probes every shard for (client_id, fid_key) before placing, so a key has one live entry table-wide"
  - "five new aead_fid pins and evidence/PINS-04-19.md"
affects: ["04-VERIFICATION.md WIRE-08 re-check", "04-23 gap-closure review checkpoint (port/src diff)", "04-24/04-25 kernel byte evidence (port/src changed)"]

actuals:
  tokens: 6138
  tasks: 2
  commits: 6
  plan_head_before: 3041ded324249882e0dfd4af33da718711bd6eb1
  plan_head_after: f6c07b08a511d2fbeb0b205240fe7bab30869868

tech-stack:
  added: []
  patterns:
    - "D-03 / IG-V01: RED commit (tests + a behaviour-preserving stub), GREEN fix, GREEN run twice, pin-advance ledger commit"
    - "A guard that must not cost a read is register arithmetic on the handle and arguments before the single deref_tiny"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-19.md
  modified:
    - port/src/fid.rs
    - port/tests/aead_fid.rs

key-decisions:
  - "Client id 0 is reserved as the cleared-slot identity; fid number 0 stays usable by every nonzero client (WR-04)"
  - "resolve refuses a ptr above HOME_SLOTS, because the table's shards are home-only and ds folds a wider pointer onto another slot (WR-06)"
  - "insert probes every shard with lookup(key, same) before the placement walk, so a fallover cannot leave two live entries (WR-09)"
  - "All three guards are register arithmetic on the handle, the arguments and the eight words already read; resolve is still one deref_tiny, zero header reads and no lock"

patterns-established:
  - "Pin ledger PINS-04-19.md: eight PIN lines, two pins, RED panic lines quoted verbatim"
  - "A torn-read pin is a hand-built word image plus the rule that makes it unresolvable, never a reproduced race"

requirements-completed: [WIRE-08]

coverage:
  - id: D1
    description: "Client id 0 is reserved: insert and resolve refuse it, and a cleared or torn identity word matches no client or handle"
    requirement: WIRE-08
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_resolve_torn_clunk_image_never_matches"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_insert_refuses_client_id_zero"
        status: pass
    human_judgment: false
  - id: D2
    description: "A handle whose ptr is above ds::tpht::HOME_SLOTS is refused by resolve, clunk and mark_reclaimed with zero arena reads and unchanged bitmaps"
    requirement: WIRE-08
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_resolve_out_of_window_ptr_is_rejected"
        status: pass
    human_judgment: false
  - id: D3
    description: "One live entry per (client_id, fid_key) across every shard, including after a fallover"
    requirement: WIRE-08
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_insert_duplicate_after_fallover_is_refused"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_insert_same_key_twice_in_one_shard_is_refused"
        status: pass
    human_judgment: false
  - id: D4
    description: "resolve stays exactly one arena read and zero header reads with no lock"
    requirement: WIRE-08
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_resolve_is_one_arena_read"
        status: pass
    human_judgment: false

# Metrics
duration: 14min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 19: fid-table guards — the torn clunk, the out-of-window pointer, the fallover duplicate Summary

**Three fid-table verification gaps closed by register-only guards: client id 0 reserved so a cleared/torn identity resolves nothing, a `ptr` above `HOME_SLOTS` refused before the arena read, and `insert` probing every shard so one key has one live entry table-wide — resolve still one `deref_tiny`, zero header reads, no lock.**

## Performance

- **Duration:** 14 min
- **Started:** 2026-11-24T23:31:00-05:00
- **Completed:** 2026-11-24T23:45:00-05:00
- **Tasks:** 2 (both `tdd="true"`)
- **Files modified:** 3 (2 code/test, 1 evidence ledger), 356 insertions / 26 deletions

## Gap IDs closed

| Gap | Closed by | Pin id | ADVANCED commit |
|-----|-----------|--------|-----------------|
| WR-04 | client id 0 reserved; `FidEntry::matches` rejects client 0 and a zero identity word | `gap-wr04-wr06-resolve-guards` | `a9be163` |
| WR-06 | `resolve` refuses a `ptr` above `ds::tpht::HOME_SLOTS` before the read; `clunk`/`mark_reclaimed` inherit it | `gap-wr04-wr06-resolve-guards` | `a9be163` |
| WR-09 | `insert` probes every shard for `(client_id, fid_key)` before the placement walk | `gap-wr09-fallover-duplicate` | `e0f7427` |

## Accomplishments

- **WR-04.** Client id 0 — the identity a cleared arena slot reads as — is refused by `insert` and `resolve` before any shard is touched, and `FidEntry::matches` additionally rejects an all-zero identity word. Every live entry's word 0 is therefore non-zero whatever its fid number, so the hand-built torn image (word 0 zero, the epoch word and `FID_LIVE` still set) matches no client and no handle. Fid number 0 stays usable by every nonzero client.
- **WR-06.** `resolve` rejects `handle.ptr` above `ds::tpht::HOME_SLOTS` (16) after the shard-range check and before `deref_tiny`. `clunk` and `mark_reclaimed` resolve first, so neither ever computes a slot id from an unvalidated pointer. `StatefulHandle::from_wire` and the 8-byte wire form are unchanged: the wire still accepts 1..=255, and the table refuses what its home-only shards cannot have issued.
- **WR-09.** `insert` calls `lookup(key, same)` on every shard before the placement walk; a hit anywhere returns the existing "already present" error and changes no shard and no bitmap. `resolve` is untouched.
- **The single-read property holds.** `resolve` contains exactly one call into a shard (`deref_tiny`), no bitmap access and no lock; `aead_fid_resolve_is_one_arena_read` still asserts `(arena_reads, header_reads) == (1, 0)` and passes.

## Task Commits

Each task was committed atomically (test → fix → ledger docs, the plan 04-09/04-10 pin precedent):

1. **Task 1 RED: torn clunk image, client id 0, out-of-window pointer** — `38acbb5` (test)
2. **Task 1 GREEN: reserve client id 0, refuse pointers outside the home window** — `a9be163` (fix)
3. **Task 1 ledger: advance `gap-wr04-wr06-resolve-guards`** — `7273fee` (docs)
4. **Task 2 RED: duplicate insert after a fallover** — `32ab8cc` (test)
5. **Task 2 GREEN: `insert` probes every shard before placing** — `e0f7427` (fix)
6. **Task 2 ledger: advance `gap-wr09-fallover-duplicate`** — `f6c07b0` (docs)

**Plan metadata:** the SUMMARY commit follows (docs).

_Note: TDD tasks have multiple commits (test → fix → ledger)._

## Files Created/Modified

- `port/src/fid.rs` — `FidEntry::matches`; client id 0 reserved in `insert`/`resolve`; `ptr > HOME_SLOTS` refused in `resolve`; all-shard duplicate probe in `insert`; module-doc `# Client id 0 is reserved`; updated `# client_id has no authenticated source yet` and the `clunk`/`mark_reclaimed`/`insert` doc comments. No `unsafe`, no `spin`, no `unsafe impl`.
- `port/tests/aead_fid.rs` — five new pins: `aead_fid_resolve_torn_clunk_image_never_matches`, `aead_fid_insert_refuses_client_id_zero`, `aead_fid_resolve_out_of_window_ptr_is_rejected`, `aead_fid_insert_duplicate_after_fallover_is_refused`, `aead_fid_insert_same_key_twice_in_one_shard_is_refused`. 24 → 29 `aead_fid_*` tests.
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-19.md` — the ledger, two pins, eight PIN lines, RED panic lines quoted.

## Decisions Made

- **Client id 0 is the reserved cleared-slot identity.** It is what `from_words` of an all-zero image yields, so refusing it makes the rule "every live entry's identity word is non-zero" true for free. Recorded in the module doc as `# Client id 0 is reserved`.
- **The WR-06 check is `ptr > HOME_SLOTS`, placed after the shard-range check and before the read.** Placing it before the read is what keeps `clunk` and `mark_reclaimed` from ever computing a slot id from a folded pointer while costing zero extra reads.
- **The WR-09 probe uses the shard's own `lookup(key, same)` rather than a shared bitmap or index.** One lookup per shard on the insert path only; `resolve` is untouched.

## The torn read is pinned by its image, not reproduced as a race

`aead_fid_resolve_torn_clunk_image_never_matches` hand-builds the eight words a resolve racing a `clunk` can observe — word 0 (fid_key, client_id) already zeroed, word 5 holding the table's epoch and `FID_LIVE`, every other word zero — and asserts `FidEntry::matches` is false for client id 0 and for every nonzero client. **This is not a concurrency proof.** The race is between `ds`'s word-by-word clear and a relaxed eight-word load and is not reproduced on the host; the pin identifies the image and the rule that makes it unresolvable, exactly as the plan's flagged assumption states.

## WR-06: which pointer the old code accepted, and what it did with it

The RED panic line quoted in the ledger is the folded pointer: `resolve must refuse the folded ptr 65`, i.e. the issued `ptr` 1 plus 64 (16 times the four blocks of the shard), which `ds` folds with `((block << 4) + (ptr - 1)) & slot_mask` (mask 63) back onto the handle's own slot. Before the fix, `resolve` read that slot through `deref_tiny` and **returned the live entry**; `clunk` and `mark_reclaimed` then resolved first and computed the slot id with the same fold, so for an accepted pointer the slot id was the handle's own. The copies with `ptr` 17 and 255 were already refused before the fix by the identity comparison, after one arena read each. The pin therefore shows that the old code accepted exactly one pointer the table never issued — the folded one — and returned the handle's own entry for it; it claims nothing beyond that observation (in particular, no second slot id was ever observed for one handle).

## Open question for the owner (recorded, not decided here)

**Two inserts of one `(client_id, fid_key)` racing from two threads are not serialised by the WR-09 probe.** The probe covers a *repeated* insert after a fallover, which is WR-09's truth. Two inserts racing at the same instant could both pass the probe and land in different shards. Plan 04-22 puts the shard mutation and its bookkeeping under one lock; whether two racing inserts of one key into two different shards also need a table-level rule is left to the owner, per the plan's flagged assumption.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. `aead_negotiate`'s end-to-end server uses client ids 1 and 2, so reserving 0 needed no change outside `files_modified`.

## Known Stubs

None.

## Threat Flags

None — no new network endpoint, auth path, file-access pattern or schema change; the three guards only narrow what an existing handle may resolve.

## Next Phase Readiness

- WIRE-08's three partial gaps (WR-04, WR-06, WR-09) are pinned RED-then-GREEN with the ledger `evidence/PINS-04-19.md`; the one-read resolve pin is unchanged and green; `cargo xtask test`/`check`/`clippy` all exit 0 and `aead_fid` holds 29 passing pins.
- WR-05 (the sweep) is plan 04-22's and is untouched here; the WR-09 race question above is the only item this plan carries forward.
- `port/src` changed, so kernel bytes change: plans 04-24 and 04-25 must redo the byte evidence and the owner stop, and the whole gap-closure `port/src` diff is seen at plan 04-23's review checkpoint.
- Review findings outside the gaps (IN-01's `port/src/lib.rs` sentence, IN-06, IN-07) were not touched and stay `open`.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- `FOUND: port/src/fid.rs`
- `FOUND: port/tests/aead_fid.rs`
- `FOUND: .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-19.md`
- `FOUND: .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-19-SUMMARY.md`
- Commits `38acbb5`, `a9be163`, `7273fee`, `32ab8cc`, `e0f7427`, `f6c07b0` are all ancestors of HEAD
- `cargo xtask test` (twice), `cargo xtask check`, `cargo xtask clippy` exit 0; `aead_fid` 29/29 and `aead_negotiate` 10/10 pass
- Ledger pin-line count: 8
