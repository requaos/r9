# Phase 4 gap-closure `port/src` review (plan 04-23 Task 2, D-24)

This is the one place the owner sees the whole gap-closure `port/src` change of Phase 4. D-24
waived the standing diff-first rule for this phase on the condition that the review happens at
phase end, before anything is pushed; a code change after the closing evidence voids its hashes,
so this review comes before plan 04-24's kernel byte evidence and plan 04-25's push. Read what
follows, run the commands in "How to read the change", and answer **approve** or **changes: …**.
Nothing is pushed by this plan.

The range end is the **insert-lock amendment commit** the owner asked for after reading the first
cut of this diff: the two racing inserts this bundle listed as an open question are resolved at it
(point 12 below), and all five pre-review legs were re-run at it — `evidence/closing-suite-gaps.md`
holds those verdicts and marks the earlier `a669317` block superseded. `port/src` still holds
exactly two changed files.

## The range

- GAPBASE `5d1549af07de7220d6f48aac0d0db221aa7c541c` — the commit `origin/main` holds, the source
  the owner approved and gated in plans 04-14 and 04-15.
- HEAD `6fd201c072d9c42f4de6e6d617c0ac35cf20d013` — the insert-lock amendment commit the
  owner asked for after reading this diff
  (`fix(04-23): serialise FidTable inserts so a racing pair cannot both land (WR-09)`), which
  is the last source change in the range. Task 1's
  base check proved the first gap-closure source commit `38acbb5` has a parent (`3041ded`) with
  no source difference from `GAPBASE`, so everything below is the gap closure and nothing else.

Everything below is `GAPBASE..HEAD`.

## What changed, by the numbers

```
$ git diff --stat 5d1549af07de7220d6f48aac0d0db221aa7c541c 6fd201c072d9c42f4de6e6d617c0ac35cf20d013 -- port/src
 port/src/chan/transport.rs | 483 +++++++++++++++++++++++++++++++++++++++++----------
 port/src/fid.rs            | 291 ++++++++++++++++++++++-----
 2 files changed, 626 insertions(+), 148 deletions(-)
```

`git diff --name-only GAPBASE HEAD -- port/src` prints exactly these two files:
`port/src/chan/transport.rs` and `port/src/fid.rs`.

## The gap findings and where each was fixed

One row per gap finding, plus one row for the insert-lock amendment the owner asked for. Every
ledger pin named here was RED against the pre-gap-closure code (or is a recorded guard) and GREEN
now, and each pin's ledger line names the `ADVANCED` commit recorded in `04-REVIEW-DISPOSITION.md`;
the amendment row names a test rather than a ledger pin, and its record is point 12 below plus
`evidence/closing-suite-gaps.md`.

| Finding | File | Function or item changed | Pin that proves it | Plan |
|---|---|---|---|---|
| CR-01 | `port/src/chan/transport.rs` | `Rekey::fail` drops the exchange secret and zero-fills and clears `Rekey::out`; `Session::keys_zeroized` and `Session::exchange_secret_zeroized` read that storage | `gap-cr01-exchange-secret` | 04-18 |
| CR-02 | `port/src/chan/transport.rs` | `RekeyIncoming`, `is_rekey_traffic` and the tag-carrying `rekey_step`/`rekey_handle` routing | `gap-cr02-wr07-routing` | 04-21 |
| CR-03 | `port/src/chan/transport.rs` | `exchange_frame`: the transcript absorbs the unpadded standard 9P2000 frames | `gap-cr03-unpadded-transcript` | 04-21 |
| WR-01 | `port/src/chan/transport.rs` | `handle_ack` returns this direction to `Idle`; `rekey_due` adds the two pending-slot conditions and the no-deadline condition | `gap-wr01-wr02-rekey-lifecycle` | 04-18 |
| WR-02 | `port/src/chan/transport.rs` | `SendHalf::send` checks the stored deadline against `now` inside the send lock, before sealing | `gap-wr01-wr02-rekey-lifecycle` | 04-18 |
| WR-03 | `port/src/chan/transport.rs` | `seal_packet` and `open_packet` route the body through `crypt_body` (one P5 layout with `Session`) | `gap-wr03-helper-keystream` | 04-18 |
| WR-04 | `port/src/fid.rs` | `FidEntry::matches`; client id 0 reserved in `insert` and `resolve` | `gap-wr04-wr06-resolve-guards` | 04-19 |
| WR-05 | `port/src/fid.rs` | `ActiveSlots` (slot ids plus the recorded TPHT key); `sweep` removes by the recorded key; shard mutation and bookkeeping under one `active` lock | `gap-wr05-sweep-recorded-key` | 04-22 |
| WR-06 | `port/src/fid.rs` | `resolve` refuses `handle.ptr` above `ds::tpht::HOME_SLOTS` before the arena read; `clunk`/`mark_reclaimed` inherit it | `gap-wr04-wr06-resolve-guards` | 04-19 |
| WR-07 | `port/src/chan/transport.rs` | `REKEY_TAG` is `NOTAG` on `Trekey`/`Rrekey` and on every exchange control message | `gap-cr02-wr07-routing` | 04-21 |
| WR-08 | `xtask/src/test9p.rs` | `command_specs` one leg per listed `port/tests` file; `check_leg_output` every result line must report tests | `gap-wr08-per-file-legs` | 04-20 |
| WR-09 | `port/src/fid.rs` | `insert` probes every shard for `(client_id, fid_key)` before the placement walk | `gap-wr09-fallover-duplicate` | 04-19 |
| WR-09 (racing inserts, owner request) | `port/src/fid.rs` | `FidTable::insert_lock` (`"fid-insert"`): `insert` holds a **table-level** lock from before the all-shard probe until it returns, so the probe and the placement walk are one indivisible step and two concurrent inserts of one key cannot both pass the probe; lock order `insert_lock` → `active` → `reclaimed`; `resolve` still takes no lock | `aead_fid_insert_racing_pair_lands_exactly_one_entry` (32 scoped threads, 64 rounds: two winners in two shards without the lock, exactly one with it) | 04-23 |

`xtask/src/test9p.rs` is outside `port/src` (the host-only xtask crate, linked into neither kernel
image), so it needs no approval under the standing rule; it is listed because WR-08 is a gap
finding and the closing evidence re-runs it.

## How to read the change

Run from the repository root; each is read-only.

```
git diff 5d1549af07de7220d6f48aac0d0db221aa7c541c 6fd201c072d9c42f4de6e6d617c0ac35cf20d013 -- port/src
git diff 5d1549af07de7220d6f48aac0d0db221aa7c541c 6fd201c072d9c42f4de6e6d617c0ac35cf20d013 -- port/tests
git diff 5d1549af07de7220d6f48aac0d0db221aa7c541c 6fd201c072d9c42f4de6e6d617c0ac35cf20d013 -- xtask/src
git diff 5d1549af07de7220d6f48aac0d0db221aa7c541c 6fd201c072d9c42f4de6e6d617c0ac35cf20d013 -- .planning/rfc-9p2000-aead.md .planning/PROJECT.md .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-WIRE-DECISIONS.md
```

The supporting ranges outside `port/src`: `port/tests` +2,365/−25 (`aead_fid.rs` +795/−3,
`aead_negotiate.rs` +169/−2, `aead_transport.rs` +1,295/−7, `common/aead_link.rs` +106/−13),
`xtask/src/test9p.rs` +133/−26, and the three planning documents plan 04-21 edited
(`rfc-9p2000-aead.md` +4, `PROJECT.md` +3/−1, `04-WIRE-DECISIONS.md` +5/−3).

## The points worth your eye

Every line number is at HEAD `6fd201c`.

1. **The scrub in `Rekey::fail`, and what `keys_zeroized` now covers — `port/src/chan/transport.rs:994`, `:1024`, `:1036`.** `Session::fail` (`:987`) calls `self.rekey.fail()`; the re-key state drops the exchange secret (`Key` is `Zeroizing`), zero-fills and clears `out`, and drops the transcript. `keys_zeroized` (`:1024`) is `send.keys_zeroized() && recv_keys.is_zeroized() && rekey.keys_zeroized()`; `exchange_secret_zeroized` (`:1036`) is `secret.is_none() && out.iter().all(== 0)`. Both read storage, never a flag, so a failure path that skipped the scrub answers `false`. The `Rekey::keys_zeroized` body (`:562`) covers the pending and superseded slots plus those same two conditions.
2. **The idle reset in `handle_ack` and the two new conditions of `rekey_due` — `transport.rs:1350` (reset at `:1370-1375`), `:1047` (conditions at `:1051-1053`).** A verified ACK zeroizes the superseded keys, clears the deadline, and sets `ready`, `own_marker_sent`, `request_sent`, `active` false with `phase = ExPhase::Idle`, so a later threshold crossing starts another exchange. `rekey_due` also requires both `pending` slots empty and no outstanding deadline, which is what stops a new exchange overlapping the previous generation.
3. **The deadline check inside the send lock of `SendHalf::send` — `transport.rs:402`, check at `:422`.** The check sits after the lock (`:417`) and before `seal_into`, so a withheld acknowledgement writes nothing; the seal, the write and the counter advance are in the same lock scope (D-21).
4. **`is_rekey_traffic` and the three outcomes of `rekey_step` for an incoming message — `transport.rs:116`, `:1091`, `rekey_handle` at `:1152`.** `rekey_step` with `incoming = Some(..)` hands the message to `rekey_handle`, which: (a) dispatches reserved-tag exchange/control traffic that fits the route; (b) **closes through `fail()` + `Err(Closed)`** on a `NOTAG` message that does not fit the route (wrong fid, wrong kind, or a `NOTAG` compound the exchange never dispatches); (c) returns `Ok(())` with **no packet and no state change** for any other tag — ordinary traffic crossing an exchange is never consumed. A separate branch at `:1101` closes a session whose shared send half closed itself (WR-02).
5. **The tag and afid constants against `DECISION P9` and P3 — `transport.rs:70` (`REKEY_AFID = 0xFFFF_FFFE`), `:81` (`REKEY_TAG = crate::devmnt::NOTAG`), afid checks at `:122` and `:126`, all six exchange messages tagged `REKEY_TAG` (`:1390`, `:1501`, `:1512`, `:1528`).** `is_rekey_traffic` also puts `Tstream`/`Rstream` (`:128-135`) and `Tcompound`/`Rcompound` (`:135`) in exchange space only under `NOTAG` (the owner's two P9 amendments).
6. **`exchange_frame` against P2 — `transport.rs:1687`.** It writes `size[4] type[1] tag[2]` then the standard body, with `Twrite`, `Rread` and `Rstream` writing the true count and exactly that many data bytes, unpadded, so a 21-byte `Twrite` is absorbed as 44 bytes and an empty one as 23. The sealed body still comes from `msg::encode_body` (P4, page-padded), so only what is hashed changed.
7. **The helpers through `crypt_body` — `seal_packet` at `transport.rs:1773` (`:1798`), `open_packet` at `:1815` (`:1849`), `crypt_body` at `:1736`.** Parameters and non-data bodies from ChaCha20 block counter 0, data page `i` from `1 + 64 * i`, the same placement `Session` uses (`:624`, `:926`).
8. **`FidEntry::matches` and the reserved client id 0 — `port/src/fid.rs:350`, `:681` (insert), `:763` (resolve).** Client id 0 is the cleared arena slot's identity, so refusing it makes every live entry's identity word non-zero; a torn or cleared image (word 0 zero, epoch and `FID_LIVE` still set) matches nobody. Fid number 0 stays usable by every nonzero client.
9. **The home-window pointer check — `fid.rs:773`.** `resolve` refuses `handle.ptr` above `HOME_SLOTS` (16) after the shard-range check and before the one `deref_tiny`, because the table's shards are home-only and `ds` folds a wider pointer onto another slot; `clunk` and `mark_reclaimed` resolve first, so neither computes a slot id from an unvalidated pointer. `resolve` still takes no lock and stays one arena read.
10. **The all-shard duplicate probe — `fid.rs:710-716`.** `insert` calls `shard.tpht.lookup(key, same)` on every shard before the placement walk, so a key inserted before a fallover cannot gain a second live entry; a hit returns the existing "already present" error and changes no shard and no bitmap. The probe and the walk it licenses are one critical section of the table's `insert_lock` (point 12).
11. **`ActiveSlots`, the lock order and the sweep by recorded key — `fid.rs:513`, insert `:724-730`, clunk `:804` (active then reclaimed), sweep `:906` (locks `:910`/`:912`, removal `:926-934`).** The live set is the dense slot ids plus the TPHT key recorded at insert. `sweep` takes each collected id's **recorded** key and removes that key's entry; the slot's current contents are never read, so a slot clunked and reused during a sweep keeps the live newcomer, and a slot that reads as zeros can never name key `(0, 0)`. Every mutator takes `active` before `reclaimed`; `resolve` takes neither.
12. **The table-level insert lock — `fid.rs:581` (the field), `:693` (taken), module note at `:70-84`.** `insert` takes `insert_lock` (a `Lock<()>` named `"fid-insert"`) after the client-id-0 refusal and holds it across the whole all-shard probe **and** the placement walk, so the absence the probe proves cannot be invalidated before the placement it licenses. This is the case WR-09's probe alone left open and the owner asked for: with the probe's read and the walk indivisible, two concurrent inserts of one `(client_id, fid_key)` cannot both pass the probe and land in different shards. The order is `insert_lock` → `active` → `reclaimed` and only that way; `clunk`, `mark_reclaimed`, `sweep` and `bitmap_lens` take no `insert_lock`, so it never nests inside a shard lock, and `resolve` is untouched — no lock, one arena read. The pin is `aead_fid_insert_racing_pair_lands_exactly_one_entry` in `port/tests/aead_fid.rs`: 32 scoped threads insert one key for 64 rounds while another thread frees a slot in that key's home block, and it asserts one `Ok`, every other insert the "already present" error, one live resolvable entry and a sweep that leaves nothing stale. Without the lock it reports two winners in two shards within a few rounds.

## The owner's `DECISION P9` line, quoted

> DECISION P9: amended (2026-11-25) — accepted as proposed (tag NOTAG 0xFFFF on all six exchange messages, afid 0xFFFFFFFE), with the owner's addition that Tstream, Rstream, Tcompound and Rcompound are covered by the same reserved routing: a sealed Tstream or Rstream is exchange traffic only when its tag is NOTAG, and for Tstream its FidHandle names the exchange afid 0xFFFFFFFE; Rstream carries no fid, so NOTAG plus its type identifies it; Tcompound and Rcompound carry no top-level fid, so NOTAG plus the type alone identifies them as exchange-space; any message in this family with any other tag is ordinary traffic and the re-key state machine never consumes it, and a NOTAG message the exchange does not expect closes the connection fail-closed rather than being delivered as ordinary traffic. If the exchange ever carries one of these types, it enters the P2 transcript in the same standard 9P2000 body form with the data unpadded. P4 is unchanged.

P3 is unchanged: `Trekey = 138` / `Rrekey = 139`, `NOTAG`, zero `FidHandle`, `kind`/`generation`/`confirm`, the `1 << 48` packet threshold and the 5000 ms ack timeout. `PROPOSAL P9:` is untouched in `04-WIRE-DECISIONS.md`; the `DECISION` line is the answer that binds.

## Deviations and open questions the five summaries recorded

### Deviations inside the reviewed range

| Plan | Deviation | Kind | Commit |
|---|---|---|---|
| 04-18 | The Task 2 "pending-keys `Tauth` closes" pin passes at RED: the pre-fix state machine cannot reach `Idle` with a pending slot, so the pin guards the GREEN-reachable state the idle reset creates. Kept and recorded in the ledger rather than weakened. | Rule 1 test design (no code impact) | `087a4da` |
| 04-18 | Three `docs(04-18)` pin-advance commits, so each ledger `ADVANCED` line names its own fix commit | process | `073605f`, `a741806`, `54848d4` |
| 04-19 | None — plan executed as written | — | — |
| 04-20 | None — the ledger `ADVANCED` line rides the closing metadata commit because the GREEN hash cannot be known inside it | process note | — |
| 04-21 | The fixture keeps a `NOTAG`-forwarding `rekey(exchange, Option<&Msg>)` convenience and adds `rekey_tagged`/`recv_tagged`, rather than moving all 77 call sites | test design | `4bb286a` |
| 04-21 | Three pin-advance commits beyond the plan's named commits (same ledger reason as 04-18) | process | `11d8c83`, `ed0065c` |
| 04-21 | `aead_transport_rekey_stream_ordinary_is_not_consumed` passes at RED — the catch-all already ignored `Tstream`/`Rstream`, so it guards the owner's amendment rather than changing it | RED signal note | `4bb286a` |
| 04-22 | None — plan executed as written | — | — |

No deviation changes a wire byte or a public signature. The plan 04-21 compound amendment
(`2242575`) is in-range source, not a deviation: it is the owner's second P9 amendment.

### Open questions the summaries carry for the owner

- **RESOLVED — the two racing inserts of one `(client_id, fid_key)` (WR-09, 04-19, resolved on the
  owner's request at `6fd201c`).** This was an open question in the first cut of this bundle: the
  WR-09 probe covered a *repeated* insert after a fallover, and plan 04-22's single-lock rule
  covered mutation and bookkeeping per shard, so two inserts racing at the same instant could both
  pass the probe and land in different shards. The owner read this diff and required it fixed
  before approving. `FidTable` now carries a table-level `insert_lock` (`"fid-insert"`) that
  `insert` takes before the probe and holds until it returns, so the probe's read and the placement
  it licenses are one indivisible step (lock order `insert_lock` → `active` → `reclaimed`; `resolve`
  unchanged, still lock-free and one arena read). The pin is
  `aead_fid_insert_racing_pair_lands_exactly_one_entry` in `port/tests/aead_fid.rs`: 32 scoped
  threads insert one key over 64 rounds while another thread frees a slot in that key's home block;
  it asserts exactly one `Ok`, the rest the "already present" error, one live resolvable entry, and
  a sweep that leaves nothing stale. With the lock removed it reports two winners in two shards
  within a few rounds (rounds 1, 3, 5, 9, 13 and 20 observed across runs); with the lock it is
  green over repeated runs. Point 12 above is the line-by-line reading, and
  `evidence/closing-suite-gaps.md` records the re-run of all five pre-review legs at `6fd201c`.
- **The msize a re-key needs under P4 (04-21).** `DECISION P4` is unchanged and P9 says so: the sealed exchange `Twrite` is 4172 bytes (28-byte header + 4112-byte P4 body + 32-byte MAC) and the sealed exchange `Rread` is 4160. A session whose negotiated msize is below 4172 cannot re-key — `packet_len` refuses the packet, `seal_into` fails and the one `fail()` path closes. Changing that would amend P4 and is not proposed here; what this closure changed is only what the transcript hashes (44 bytes for the `Twrite`, not 4119).
- Plans 04-18, 04-20 and 04-22 record no open question for the owner.

## The seven info findings, still `open`

None of these is in the gaps block and none is dispositioned by this plan.

| Finding | Title | Status |
|---|---|---|
| IN-01 | Module docs describe the pre-implementation state | open — partially corrected by plan 04-18 |
| IN-02 | `DataLayout::pages` is computed and never used; `crypt_body` recomputes it with a literal | open |
| IN-03 | `wire::is_t_message` has no production caller | open |
| IN-04 | `Msg`'s hand-written `PartialEq` ends in `_ => false`, which hides a new variant | open |
| IN-05 | The threat-model overclaim guard only fires on rows carrying the Phase 4 status | open |
| IN-06 | The end-to-end capability test claims "zero FID memory" but does not measure it | open |
| IN-07 | The `ds` removal test's "the freed slot takes another key" is not asserted | open |

**IN-01's partial correction:** plan 04-18 rewrote the `transport.rs` module doc (`:42-47` before
the change) and the `Session::fail` doc (`:879-885` before the change) to describe the shipped
re-key state; those two paragraphs now read as fact in the file the owner reviewed. The
`chan.rs:27-31` paragraph and the `port/src/lib.rs:88-94` paragraph that IN-01 also names were not
touched and stay open, as does IN-02's duplicated page walk inside the same file.

## What this review does not cover

- Nothing is pushed by this plan; plan 04-25 offers the push after the byte evidence.
- No kernel evidence depends on these bytes yet: plan 04-24 records it at the commit this review approves, and any change wanted at this review voids it.
- The seven info findings above are the owner's to disposition.
- Peer authentication is still not in Phase 4 (D-16), and the 130–139 message pairs remain encode/decode only (semantics are Phase 6).
