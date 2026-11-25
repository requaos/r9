# Phase 4 `port/src` review (plan 04-14 Task 1, D-24)

This is the one place the owner sees the whole `port/src` change of Phase 4.  D-24 waived the
standing diff-first rule for this phase on the condition that this review happens at phase end,
before anything is pushed.  Read what follows, run the four commands in "How to read the
change", and answer **approve** or **changes: …**.  Nothing is pushed by this plan; a change
request stops the phase for a fix plan and the closing evidence waits, because a code change
after it would void its hashes.

## The range

- BASE `3ce5ed8c4e309710e2157ab5b63a03a40797d185` — the `base` line of
  `evidence/kernel-bytes/before.sha256` (the Phase 4 baseline, the last commit before this
  phase's first code).
- HEAD `39d1a29806d98fbb8aa3df2c4f240eb368109174` — the commit this review is taken at, the
  red-tree repair to `xtask/tests/threat_model.rs` on top of the STATE.md commit `b84adca`.

Everything below is `BASE..HEAD`.

## What changed, by the numbers

```
$ git diff --stat 3ce5ed8c4e309710e2157ab5b63a03a40797d185 39d1a29806d98fbb8aa3df2c4f240eb368109174 -- port/src port/Cargo.toml
 port/Cargo.toml            |    9 +
 port/src/chan.rs           |   88 +++
 port/src/chan/msg.rs       | 1077 ++++++++++++++++++++++++++++++
 port/src/chan/tlv.rs       |  319 +++++++++
 port/src/chan/transport.rs | 1567 ++++++++++++++++++++++++++++++++++++++++++++
 port/src/chan/wire.rs      |  394 +++++++++++
 port/src/devmnt.rs         |  128 +++-
 port/src/fid.rs            |  746 +++++++++++++++++++++
 port/src/lib.rs            |   14 +
 9 files changed, 4318 insertions(+), 24 deletions(-)
```

`git diff --name-only BASE HEAD -- port/src` prints exactly the eight `port/src` files below
(five new, two changed, and `port/src/lib.rs` changed); `port/Cargo.toml` is the ninth path and
carries the phase's one dependency-edge change.

## The files

| File | State at HEAD | Lines | What it does |
|---|---|---|---|
| `port/Cargo.toml` | changed | +9 | Adds the `crypto` and `ds` path dependencies (D-22, D-25): `port` is the one kernel crate that links the crypto and ds foundations, and both kernel images reach them through `port`. |
| `port/src/chan.rs` | new | +88 | The envelope module root: the 28-byte header offsets and total, `MAC_LEN`, `PAGE_LEN`, `VERSION_AEAD`, the six `ExtMask` bits and the submodule declarations — one shared pin for the four submodules. |
| `port/src/chan/wire.rs` | new | +394 | The canonical header codec (field by field with `to_le_bytes`, no layout cast), the framing bounds, the opcode registry, and the `Unverified`/`Verified` types that make "only `Size` before the MAC" a type discipline (D-06, D-07). |
| `port/src/chan/tlv.rs` | new | +319 | The `ExtTLVs` record parser: `type[1] reserved[1] len[2] value[len]` padded to 8, ascending type order, each type once, the type set equal to the mask bits, and the fixed value layouts; parses by borrow, allocates nothing (P1, D-11, D-12). |
| `port/src/chan/msg.rs` | new | +1077 | The base 9P2000 bodies unchanged plus the new 130–139 pairs, encode and decode only; the D-09 two-part data body (parameters, then whole 4096-byte pages) and the checked cursor that requires exact consumption (P4, P5, D-28). |
| `port/src/chan/transport.rs` | new | +1567 | The connection engine: the plain-framed handshake and its BLAKE3 transcript, session-key derivation, sealing and opening, the in-band re-key (marker, cutover, ack, deadline) and the single `fail()` path (D-07, D-15 to D-21, D-26). |
| `port/src/fid.rs` | new | +746 | The hybrid handle layouts (`StatefulHandle` 8 B, `StatelessCap` 24 B, `FidEntry` 64 B), the `FidHandle` discriminator, the qid codec, and `FidTable` — sharded TPHT storage whose `resolve` is one arena read (D-05, D-11 to D-14, WIRE-07/10). |
| `port/src/devmnt.rs` | changed | +128/−24 | The dialect selection point: `Dialect`, `Negotiated`, `Mnt::negotiate_dialect`; `Mnt::negotiate` keeps its signature and delegates with `Dialect::Plain`; `fversion` is unchanged (D-23). |
| `port/src/lib.rs` | changed | +14 | Declares `pub mod chan;` and `pub mod fid;`, both under the module `#[forbid(unsafe_code)]` regression lock; no other change. |

## How to read the change

Run from the repository root; each is read-only.

```
git diff 3ce5ed8c4e309710e2157ab5b63a03a40797d185 39d1a29806d98fbb8aa3df2c4f240eb368109174 -- port/src port/Cargo.toml
git diff 3ce5ed8c4e309710e2157ab5b63a03a40797d185 39d1a29806d98fbb8aa3df2c4f240eb368109174 -- crypto/src
git diff 3ce5ed8c4e309710e2157ab5b63a03a40797d185 39d1a29806d98fbb8aa3df2c4f240eb368109174 -- ds/src/tpht/flattened.rs
git diff 3ce5ed8c4e309710e2157ab5b63a03a40797d185 39d1a29806d98fbb8aa3df2c4f240eb368109174 -- xtask/src
```

The three supporting ranges are outside `port/src` and outside D-24's scope, but this phase
touched them and they are what `port` now stands on: `crypto/src/session.rs` (+604) is the key
derivation, packet MAC, nonce and re-key-confirm primitives; `ds/src/tpht/flattened.rs`
(+205/−3) is the TPHT `remove` that zeroes an arena slot; `xtask/src` gains `test9p.rs`
(+417) and the R4 rule change in `lintcontract.rs`.

## The points worth your eye

**1. `negotiate` delegates; `fversion` is unchanged — `port/src/devmnt.rs:688`, `:735`, `:1054`, `:1102`.**
`Mnt::negotiate` keeps its exact signature and body shape and now calls `negotiate_dialect(..,
Dialect::Plain)`, refusing an aead answer (`:689-692`).  The kernel's `fversion` (`:1054`)
still calls `Mnt::negotiate` (`:1102`), so a booted kernel offers plain `9P2000` and the six
Phase 4 protections are still not reached by a boot.  `negotiate_dialect` (`:735`) is where a
later phase would pass `Dialect::Aead`; the doc at `:722-734` states that, and states that a
plain answer to an aead offer is an error, not a downgrade.  The msize rules are the same three
the plain path enforced.

**2. The receive order — `port/src/chan/transport.rs:749`, order at `:763`, `:777`, `:789`, `:791`, `:808`, `:830`.**
`Session::open` takes the bytes (`:763`), bounds them, verifies the packet MAC over
`[0, size - MAC_LEN)` (`:777`) and only then decodes the header (`:789`) and checks the sequence
number (`:791`) before any other field is used.  The TLV area is parsed next (`:808`) and the
body decrypted last, after `Verified` construction (`:830`).  This is D-07's ordering and the
reason the `Unverified`/`Verified` split exists.

**3. The single `fail()` path — `port/src/chan/transport.rs:886`, reached from every rejection in `open`, `frame_len`, `receive`, `poll` and the re-key handlers.**
Every rejection in `open`, `frame_len`, `receive`, `poll` and the re-key handler goes through
the one private `fail()`: both directions' keys are overwritten with zeros, then the re-key
state's pending and superseded keys, both counters and the closed flag follow.  `teardown`
(`:898`) is the same call, so an orderly close and a protocol failure leave the same state.
There is no second error path for a reviewer to miss.

**4. The send lock scope — `port/src/chan/transport.rs:313`, lock at `:329`, write at `:335`, advance at `:338`.**
`SendHalf::send` takes the per-direction lock once (`:329`) and holds it across the seal
(`:331`), the write (`:335`) and the counter advance (`:338`), with the comment at `:332-334`
stating why: two senders sharing one half cannot put packets on the wire out of sequence.  A
short or failed write closes the direction through the same lock (`:345-347`) rather than
advancing a sequence number the peer never saw.  `Session::seal` reaches the same lock.

**5. The re-key cutover — send `transport.rs:376` (`:385`-`:392`), receive `transport.rs:1111` (`:1125`-`:1130`), keys `transport.rs:1407`.**
The send side seals the marker as the last packet under the old keys, moves those keys into the
superseded slot, installs the new keys and resets the counter to 0 — all in one lock scope
(`:385`-`:392`) — then arms the acknowledgement deadline.  The receive side replaces its keys,
holds the old ones in `recv_superseded`, zeroizes them and resets its own counter (`:1125`-
`:1130`).  `finish_exchange` (`:1407`) derives the next generation from the chained transcript
and stores the pending keys for both directions; nothing is installed until the marker arrives.
A missing ack closes on the next receive through the deadline (`:759`-`:761`).

**6. `FidTable::resolve`'s comparisons — `port/src/fid.rs:602`, comparisons at `:612`-`:616`.**
`resolve` bounds the shard id, derives the TPHT key from `client_id` and the handle's `fid_key`,
and does exactly one arena read (`deref_tiny`, `:610`) — zero header reads.  Every comparison is
register arithmetic on the words just read: `fid_key`, `client_id`, `entry.epoch !=
handle.epoch`, `handle.epoch != self.epoch`, and the `FID_LIVE` bit.  A clunked handle cannot
resolve because `remove` zeroed the slot (point 7), and an epoch bump revokes every handle
issued before it.

**7. The `ds` slot clearing — `ds/src/tpht/flattened.rs:397`, zeroing at `:424`.**
`remove` zeroes the arena words **before** it clears the occupied bit (`:418-428`), for the
reason stated there: a clunked `StatefulHandle` still names that `(key, tiny_ptr)` pair and
`deref_tiny` reads the arena without consulting the header, so leaving the words behind would
let the handle keep resolving.  The stores are safe `Release` stores inside the block's seqlock
section.

**8. The R4 rule change — `xtask/src/lintcontract.rs:18`, `:336`, `:341`.**
R4 was "no kernel crate depends on `crypto` or `ds`".  It is now "only `port` may, in any
dependency table; the three arch crates may not name either key and reach them through
`port`".  The change is the `if kernel == "port" { continue; }` at `:341` plus the message
rewording; the dev-dependency table stays a dependency table for arch crates, and the accepting
case is covered by a test.  This is the one *rule* changed in the phase, and it changed in the
same commit as the `port/Cargo.toml` edge it permits.

## The eight `DECISION P` outcomes

All eight were accepted by the owner on 2026-11-25 (`04-WIRE-DECISIONS.md:19-26`), and each was
pinned byte for byte by a plan; the `PROPOSAL Pn` value stands where decision and proposal
agree, and this phase's code follows the accepted text.

| Decision | Outcome | What it fixed |
|---|---|---|
| P1 | accepted | The sixth `ExtMask` bit, `EXT_STATELESS_CAP = 1 << 5`, and the TLV record layout, order, uniqueness and fixed lengths. |
| P2 | accepted | Session derivation: HKDF-Extract over the transcript, the 18-byte `info`, and the transcript chain across a re-key (plain-framed handshake messages and re-key exchange messages only). |
| P3 | accepted | The re-key messages: `Trekey = 138` / `Rrekey = 139`, `NOTAG`, zero `FidHandle`, `kind`/`generation`/`confirm`, and the `1 << 48` packet threshold, 5000 ms ack timeout. |
| P4 | accepted | Message bodies: base 9P2000 bodies byte for byte; the server-issued handle travels in the header `FidHandle`; the data messages carry parameters then padded pages. |
| P5 | accepted | Keystream placement: parameters and non-data bodies from ChaCha20 counter 0, data page `i` from `1 + 64 * i`. |
| P6 | accepted | `blake3`'s `zeroize` feature stays off, and the per-packet hasher's key-copy limit is stated in the `session.rs` module doc. |
| P7 | accepted | A packet whose sequence number is not the expected next value closes and zeroizes the connection — not a drop alone. |
| P8 | accepted | The `StatelessCap` tag: 8 bytes of keyed BLAKE3 over `r9 StatelessCap1` and the capability's first 16 bytes, at the table's epoch; a failed verification closes the connection. |

`DECISION IG-D11` (not one of the eight) was lock-amended with the proposed wording unchanged.

## Deviations the plan summaries recorded

These are the unplanned changes inside the reviewed range, as the thirteen summaries record
them.  None changes a wire byte or a public signature; the aggregate is one dependency edge,
two observability accessors, one lint-rule change (point 8) and a set of local fixes.

| Plan | Deviation | Kind |
|---|---|---|
| 04-02 | `seal_packet`'s zero-fill rewritten as `vec![0u8; size]` (clippy); `Size` overflow past `u32::MAX` now returns an error instead of truncating | 1 bug, 1 missing-critical |
| 04-03 | None | — |
| 04-04 | `is_multiple_of` instead of `%` in `wire.rs` (clippy) | 1 lint |
| 04-05 | `Msg` hand-implements `PartialEq`/`Eq` because `dat::Qid` does not derive it and `dat.rs` was read-only; two `as u16` length casts became `u16::try_from` | 1 blocking, 1 bug |
| 04-06 | A `docs` commit per task to advance the pin ledger (the `ADVANCED` line cannot name its own commit) | process |
| 04-07 | `pub(crate) Unverified::take` added so the mandated `Session::open` signature can verify and decrypt in place; the test client's `recv` drained the wrong queue | 1 blocking, 1 bug |
| 04-08 | Two Task 1 pins updated to accept the server's ack before the next packet, which is the real wire order | 1 blocking (test-only) |
| 04-09 | Dropped a no-op `& 0xFF` mask on a `u8` (`gpu_qid_path`) that clippy flagged; extra `docs` pin commits | 1 bug, process |
| 04-10 | `FidTable::home_shard` and `FidTable::bitmap_lens` added — read-only accessors the plan's own pins need; extra `docs` pin commits | 1 missing-critical, process |
| 04-11 | No production fix; `negotiate_dialect` does the version exchange on the io halves rather than through a throwaway `Mnt` (the `BusyCell`/`forbid(unsafe_code)` constraint), bytes identical | 0, plan-structure note |
| 04-12 | The measured `port` unsafe count is 22, not the planned 23; `preflight` covers all seven listed files rather than six | 2 bug/consistency |
| 04-16 | None; the only non-plan text is `#[allow(clippy::too_many_arguments)]` on the two mandated signatures and removal of two now-unread `role` fields | — |
| 04-17 | None | — |

## Red-tree repair

The tree was red at the HEAD this plan was dispatched on, and it is green now because of one
test-only commit inside the range:

- `038ef15` (plan 04-13, owner-authorized D-23) moved the six Phase 4 section-5 rows of
  `docs/threat-model.md` from `not implemented` to `implemented and host-tested in Phase 4; not
  used by a booted kernel (D-23)`.
- `39d1a29` (this plan's fix, deviation Rule 1) updates the pin that still demanded
  `not implemented`: `xtask/tests/threat_model.rs` now accepts either status and gains the
  inverse guard the document now relies on — a row carrying the Phase 4 status may not claim
  the code is *used* by a booted kernel.  The test is renamed
  `threat_model_protections_are_not_overclaimed`.

Why it does not touch this review: `xtask` is a host-only workspace crate.  It is linked into
neither kernel image, so `port`, `crypto`, `ds` and both kernels are byte-identical to the
commit before the repair, and the baseline in `evidence/kernel-bytes/before.sha256` still
compares.  The change is in `xtask/tests/` only — no source file outside it moved in this plan,
and `.planning/STATE.md`'s open-question rows (D-10) are the only other edit.

## What this review does not cover

- Nothing is pushed here; plan 04-15 offers the branch push after the closing evidence.
- Peer authentication is not in Phase 4 (D-16): the session secret is injected behind
  `SessionExchange` and no live authenticated connection between two real machines comes out of
  this phase.
- The new 130–139 message pairs are encode and decode only; their semantics are Phase 6.
- The capability-revocation and failover-reclaim rows of the threat model remain unimplemented.
