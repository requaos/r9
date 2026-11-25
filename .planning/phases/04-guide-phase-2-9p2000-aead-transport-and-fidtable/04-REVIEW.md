---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
reviewed: 2026-11-24T00:00:00Z
depth: standard
files_reviewed: 26
files_reviewed_list:
  - Cargo.lock
  - crypto/src/lib.rs
  - crypto/src/session.rs
  - crypto/tests/session.rs
  - ds/src/tpht/flattened.rs
  - port/Cargo.toml
  - port/src/chan.rs
  - port/src/chan/msg.rs
  - port/src/chan/tlv.rs
  - port/src/chan/transport.rs
  - port/src/chan/wire.rs
  - port/src/devmnt.rs
  - port/src/fid.rs
  - port/src/lib.rs
  - port/tests/aead_fid.rs
  - port/tests/aead_negotiate.rs
  - port/tests/aead_tlv.rs
  - port/tests/aead_transport.rs
  - port/tests/aead_wire.rs
  - port/tests/common/aead_link.rs
  - port/tests/common/mod.rs
  - xtask/src/lib.rs
  - xtask/src/lintcontract.rs
  - xtask/src/main.rs
  - xtask/src/test9p.rs
  - xtask/tests/threat_model.rs
findings:
  critical: 3
  warning: 9
  info: 7
  total: 19
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-11-24
**Depth:** standard
**Files Reviewed:** 26
**Status:** issues_found

## Summary

The phase's crypto primitives and static wire/body codecs are in good shape: `crypto/src/session.rs`
derives, separates directions and generations, folds MAC comparisons without early exit, and the
`port/src/chan/wire.rs` / `tlv.rs` / `msg.rs` decoders use checked cursors that reject truncation,
trailing bytes, non-page data areas and nonzero padding. The `Session::open` receive order is genuinely
MAC-then-seq-then-header-then-TLV-then-decrypt, the sequence violation goes through one `fail()`
path, the marker cutover happens entirely inside the per-direction send lock, and `ds`'s
`remove` zeroes the arena slot under the block seqlock before clearing the occupied bit. I found no
path that decrypts, parses a TLV or reads a header field before the packet MAC verifies, no new
unsafe outside the documented `ds` site, and the `chan`/`fid` modules keep their
`#[forbid(unsafe_code)]`.

The defects are concentrated in the in-band re-key path, which is the newest and least independently
pinned part of the phase. Three are blockers: the exchange secret is left un-zeroized on a failure
path while `keys_zeroized()` reports "all keys zero" (contradicting the section-5 threat-model row);
the exchange messages are indistinguishable from ordinary user traffic, so any normal
`Twrite`/`Rwrite`/`Tread`/`Rread` that crosses an exchange is consumed by the state machine or closes
the connection; and the re-key transcript absorbs the page-padded body, contradicting the accepted
DECISION P2 (which says the data is absorbed unpadded) and making the exchange cost a whole 4096-byte
page plus depend on the negotiated msize. The fid table is mostly sound, but its bitmaps are keyed by
an unvalidated handle byte and its sweep can remove a live entry that reused a swept slot.

No necessity-category finding met the evidence rule in `necessity-review.md`; the `necessity:` key is
therefore omitted.

**Toolchain note:** `cargo`/`rustc` are not installed in this environment (no toolchain on `PATH`, and
`rust-toolchain.toml` pins `nightly-2026-01-30`), so nothing was compiled or run. Every finding below
comes from reading the source and the pinned decisions; I did not verify any test actually passes.
`Cargo.lock` was reviewed for the new `port -> crypto`/`ds` edges only.

## Critical Issues

### CR-01: The re-key exchange secret survives `fail()` — `keys_zeroized()` reports a lie

**File:** `port/src/chan/transport.rs:170-183` (and `:886-895`, `:914-916`, `:1288`, `:1320`, `:1378`)
**Issue:** `Rekey::fail` zeroizes only `recv_superseded` and the `pending` slots. It does not touch
`self.secret`, which is `Some(Key)` from the moment the seam returns `Done` (line 1288 on the client,
line 1320 in the second round, line 1380 on the server) until `finish_exchange` (`:1408`). If the
session fails anywhere in that window — a MAC failure, a bad TLV, a deadline expiry, or a message the
phase machine rejects — `Session::fail` (`:886-895`) runs `rekey.fail()` and the 32-byte exchange
secret is still live in the session. `Rekey::keys_zeroized` (`:180-183`) does not include `secret`, so
`Session::keys_zeroized()` (`:914-916`) returns `true` in exactly that state: the "one failure path"
zeroes everything except the most valuable value it holds, and the public invariant that exists to
prove otherwise says it did not happen. `Rekey::out` (the seam's raw output, populated for the server
at `:1378`) is likewise never cleared and is freed with its bytes intact at `:1393` (`mem::take`).

This contradicts `Rekey::fail`'s own doc ("Zeroize every key the re-key state holds",
`:168-169`), `Session::keys_zeroized`'s doc ("Whether every key this session holds is zero",
`:907-913`), and the threat-model section 5 row "Channel keys are dropped and zeroized when a channel
is closed, so captured packets stay unreadable" (`docs/threat-model.md`, marked *implemented and
host-tested*). The test suite cannot catch it because no test fails a session after the seam has
returned `Done` and before `finish_exchange`.

**Fix:**
```rust
fn fail(&mut self) {
    // Drop (not just overwrite) the exchange secret: `Key` is `Zeroizing`,
    // so assigning `None` zeroes it on drop.
    self.secret = None;
    for byte in self.out.iter_mut() {
        *byte = 0;
    }
    self.out.clear();
    self.transcript = None;
    if let Some(keys) = &mut self.recv_superseded {
        keys.zeroize();
    }
    for keys in self.pending.iter_mut().flatten() {
        keys.zeroize();
    }
}

fn keys_zeroized(&self) -> bool {
    self.secret.is_none()
        && self.out.iter().all(|&b| b == 0)
        && self.recv_superseded.as_ref().is_none_or(DirectionKeys::is_zeroized)
        && self.pending.iter().all(|slot| slot.as_ref().is_none_or(DirectionKeys::is_zeroized))
}
```
and add a test that fails a session between the seam's `Done` and `finish_exchange` (e.g. deliver a
corrupted `Rwrite` after the client's first `step`) and asserts `keys_zeroized()` plus a
`rekey_secret_zeroized()`-style accessor, so the gap cannot reopen.

### CR-02: In-band exchange messages are indistinguishable from ordinary 9P traffic

**File:** `port/src/chan/transport.rs:959-964`, `:997-1041`, `:1283-1343`, `:1353-1402`; `port/src/chan/msg.rs:107-122`
**Issue:** `Session::rekey_step` takes `incoming: Option<&msg::Msg>` and no header tag, and
`rekey_handle` dispatches on the *message type* alone: every `Msg::Tauth | Rauth | Twrite | Rwrite |
Tread | Rread` is fed to `exchange_incoming` (`:1033-1038`). The exchange's own messages carry the
ordinary opcodes 102/103/116-119 (`begin_client_exchange` `:1049-1054`, `client_exchange_incoming`
`:1295-1331`, `server_exchange_incoming` `:1357-1395`) with tags `REKEY_AUTH_TAG = 1`,
`REKEY_WRITE_TAG = 2`, `REKEY_READ_TAG = 3` (`:69-71`), and the fid `REKEY_AFID = 7` only inside the
body — which every arm ignores (`Twrite { data, .. }` at `:1368`, `Tread { .. }` at `:1391`,
`Rwrite`/`Rread` carry no fid at all). None of the arms checks the tag or the fid.

A real mount has ordinary `Twrite`/`Rwrite`/`Tread`/`Rread` in flight while a re-key runs
(`aead_transport_rekey_outstanding_requests_complete` exists precisely because they do). An ordinary
`Rwrite` reply arriving while the client is in `AwaitingRwrite` is absorbed into the transcript and
advances the exchange (`:1304-1313`) — the reply to a user write becomes the exchange's `Rwrite`. An
ordinary `Rread` arriving in `AwaitingRread` is absorbed as the exchange's final message
(`:1315-1337`) and derives generation 1 from the wrong bytes. Any other type that happens to match an
arm while the phase does not closes the connection (`_ => fail()` at `:1339-1342` / `:1398-1401`).
A server in `AwaitingTwrite` will consume a user's `Twrite` (`:1368`) whether or not its fid is
`REKEY_AFID`.

Nothing in the API lets a caller tell the two apart: `devmnt` allocates tags from 1 upward
(`MntAlloc { next_tag: 1 }`, `port/src/devmnt.rs:701`, `alloc_tag` `:481-488`), so 1, 2 and 3 are
ordinary tags in this codebase — the very first RPC uses tag 1 (`aead_negotiate.rs:464`). The plan
assumes "tags and fids are not keyed, so outstanding requests are untouched", which holds for the
cutover but not for message *routing*.

**Fix:** make the exchange self-identifying and verify it at the router:
1. Give `rekey_step` the packet tag (or a small `enum RekeyIncoming { Exchange { typ, tag, msg },
   Control(Msg), Other }` built by the driver) and have the state machine accept a message only when
   `(typ, tag, fid)` matches the step it is waiting for.
2. Use only tags the 9P allocator can never issue for exchange messages. In this tree `0` and
   `devmnt::NOTAG` (0xFFFF) are both unused by `alloc_tag`; use those for the two `Twrite`/`Tread`
   rounds (the reply echoes the tag) and NOTAG for `Tauth`/`Rauth`.
3. Additionally require `fid == REKEY_AFID` in every `Twrite`/`Tread` arm, and never treat an
   `Rwrite`/`Rread` whose tag is not the reserved exchange tag as exchange traffic.
4. Add a test that runs an ordinary `Tread`/`Rread` through a session with an exchange in flight and
   asserts the file reply is delivered, not absorbed.

### CR-03: The re-key transcript absorbs the page-padded body, contradicting DECISION P2

**File:** `port/src/chan/transport.rs:1234-1260`; `port/src/chan/msg.rs:664-673`, `:710-719`, `:776-782`
**Issue:** `absorb_exchange` builds the absorbed frame from `msg::encode_body(message)`, and for
`Twrite`/`Rread`/`Rstream` that encoder pads the data to a whole 4096-byte page (`write_padded`,
`msg.rs:776-782`). So a `Twrite` whose seam payload is 21 bytes is absorbed as
`size[4] type[1] tag[2] fid[4] offset[8] count[4] data[21] 00 × 4075` — 4119 bytes, not the 44 bytes
the accepted wire decision pins:

> "…`size[4] type[1] tag[2]` then the standard body … and the data unpadded (`Twrite` body
> `fid[4] offset[8] count[4] data[count]`, `Rread` body `count[4] data[count]`); … the page padding …
> [is] not hashed" — `04-WIRE-DECISIONS.md:11`, `DECISION P2: accepted`.

Both ends of *this* implementation agree (both re-encode with padding), so the tests pass; but a
second implementation written to the decision will derive a different generation-1 key and every
post-cutover packet will fail the MAC. The "independent bytes" test
(`aead_transport_rekey_wire_keys_and_confirm_equal_test_side_derivation`, `:1821-1975`) cannot catch
this: `test_generations` (`:1535-1559`) decrypts the *padded* body off the link and re-frames it
verbatim, so it mirrors the implementation rather than the decision.

Second consequence: because the padded exchange `Twrite`/`Rread` always carry a whole page, the
exchange packet is ~4179 bytes regardless of payload. `MSIZE_MIN` is 256
(`port/src/devmnt.rs:138`), so a session negotiated below ~4215 bytes cannot re-key at all:
`packet_len` refuses the packet, `seal_into` fails, and the one `fail()` path closes the connection.
That is a self-inflicted denial of service for any small-mtu negotiation, and it also makes the
default 8192-msize re-key spend ~8 KB of traffic per exchange.

**Fix:** build the absorbed frame from the message's true length rather than from the padded encoder:
```rust
/// The standard 9P2000 form P2 absorbs: data unpadded (DECISION P2).
fn exchange_frame(typ: u8, tag: u16, message: &msg::Msg) -> Vec<u8> {
    let mut body = Vec::new();
    match message {
        msg::Msg::Twrite { fid, offset, data } => {
            msg::p32(&mut body, *fid);
            msg::p64(&mut body, *offset);
            msg::p32(&mut body, data.len() as u32);
            body.extend_from_slice(data);            // unpadded
        }
        msg::Msg::Rread { data } => {
            msg::p32(&mut body, data.len() as u32);
            body.extend_from_slice(data);            // unpadded
        }
        other => msg::encode_body(other, &mut body).expect("a fixed-shape exchange body"),
    }
    let mut frame = Vec::with_capacity(7 + body.len());
    frame.extend_from_slice(&((7 + body.len()) as u32).to_le_bytes());
    frame.push(typ);
    frame.extend_from_slice(&tag.to_le_bytes());
    frame.extend_from_slice(&body);
    frame
}
```
and add a pin that computes the expected transcript frame by hand with a literal 44-byte `Twrite`
so the padding rule is tested, not assumed.

## Warnings

### WR-01: The re-key state never returns to idle — at most one re-key per connection

**File:** `port/src/chan/transport.rs:920-926`, `:1061-1070`, `:1162-1180`, `:1423-1426`
**Issue:** `finish_exchange` sets `rekey.ready = true`; `handle_ack` clears the deadline and zeroizes
the superseded slot but never clears `ready` (or resets `phase` to `Idle`, or re-arms
`own_marker_sent`). `start()` is the only place that clears `ready` and `request_sent`, and both call
sites of `start` are guarded by `!self.rekey.ready` (`:1018-1023`, `:984-990`), so it can never run
again. `rekey_due()` therefore returns `false` for the rest of the connection. A server that sent a
REQUEST also strands `request_sent = true` (`:1068`) forever, so a lost REQUEST is never retried.
Plan 04-08 requires the sender on a valid ACK to "return to idle"; the threat-model row "A channel
re-keys before its sequence counter reaches 2^64 - 1" is only satisfied by the connection closing at
`SEQ_HARD_LIMIT`, not by a second re-key.

**Fix:** in `handle_ack`, after `zeroize_superseded`/`clear_deadline`, set `ready = false`,
`own_marker_sent = false`, `request_sent = false`, `active = false`, `phase = ExPhase::Idle`; in
`send_request`, do not set `request_sent` (or clear it when the exchange starts). Add a test that
drives a second threshold crossing and asserts a second exchange runs (`ce.runs() == 3`).

### WR-02: `SendHalf::send` ignores the re-key acknowledgement deadline

**File:** `port/src/chan/transport.rs:312-350` (the `let _ = now;` at `:324`)
**Issue:** `Session::seal`, `Session::open` and `Session::poll` all close the session when
`deadline_passed(now)` (`:721`, `:759`, `:1438`), but `SendHalf::send` — the path a multi-threaded
connection is documented to use ("the sending half, shareable across threads", `:737`) — discards
`now` and never consults `state.deadline`. A connection that writes only through `SendHalf` (the
fixture's two-thread test does exactly that, `aead_transport.rs:934-985`) will never notice a withheld
ACK, so D-20 step 4 / T-04-27 ("A peer that never acknowledges a re-key") is not mitigated on that
path.
**Fix:** mirror `Session::seal`'s check inside `SendHalf::send` before sealing:
```rust
if state.closed { return Err(Closed); }
if state.deadline.is_some_and(|deadline| now >= deadline) {
    state.fail();
    return Err(Closed);
}
```
and add a test that withholds the ACK and calls `send_half().send(..., now = deadline)`.

### WR-03: `seal_packet`/`open_packet` use a different keystream layout from `Session`

**File:** `port/src/chan/transport.rs:1454-1476` vs `:1488-1567`
**Issue:** `Session::seal`/`Session::open` encrypt a data body through `crypt_body`, which places the
parameters at ChaCha20 block counter 0 and each 4096-byte page at `1 + 64 * i` (P5). The public
tracer helpers `seal_packet` (`keys.keystream(seq, 0, body)` at `:1513`) and `open_packet`
(`keys.keystream(header.seq, 0, buf[body_start..])` at `:1563`) run one continuous keystream from
counter 0 over the whole body. For a `Twrite`/`Rread`/`Rstream` longer than 64 bytes the two layouts
differ, and `open_packet` verifies the MAC (which does not depend on the layout) and then returns
`Ok` with mis-decrypted plaintext. The module doc calls them "thin wrappers so the tracer tests
compile", but they are re-exported from `port::chan` (`chan.rs:38-41`) and are now two incompatible
wire layouts behind one public API.
**Fix:** route both helpers through `crypt_body` (`seal_packet` needs the `typ` it already has;
`open_packet` too), or delete them from the public re-export and keep them `#[cfg(test)]`.

### WR-04: `FidTable::resolve` can accept a torn read of a clunked slot for fid 0 / client 0

**File:** `port/src/fid.rs:602-624`; `ds/src/tpht/flattened.rs:463-479`, `:397-437`
**Issue:** `resolve` does one `deref_tiny` (eight independent relaxed loads, no seqlock — stated in
`flattened.rs:457-462`) and compares `entry.fid_key`, `entry.client_id`, `entry.epoch` and
`entry.flags & FID_LIVE`. Word 0 holds `fid_key`+`client_id` atomically, but `epoch` and `flags` live
in word 5. `remove` zeroes the arena words one at a time (`flattened.rs:424`), so a reader racing a
clunk can see word 0 *already zeroed* and word 5 *not yet zeroed* — i.e. `fid_key == 0`,
`client_id == 0`, `epoch == self.epoch`, `flags & FID_LIVE != 0`. Every one of those comparisons then
passes for a handle with `fid_key == 0`, `client_id == 0` and the table's epoch (epoch 0 is legal and
used throughout `aead_fid.rs`), so a clunked entry resolves. Nothing rejects `fid_key == 0` or
`client_id == 0` (`insert`, `:551-554`, accepts both), and the phase explicitly has no authenticated
source for `client_id` (`fid.rs:73-77`), so 0 is a plausible first-connection id.
**Fix:** reject the all-zero identity at both ends: in `insert`, return an error for `client_id == 0`
or `fid_key == 0`; in `resolve`, require `entry.fid_key != 0 || entry.client_id != 0` (or, simpler and
stronger, require the entry's word 0 to be nonzero before trusting the rest). Better still, compare a
per-slot generation word so a torn read is always detected.

### WR-05: `sweep` removes whatever the slot now holds — a reused slot's live entry is destroyed

**File:** `port/src/fid.rs:710-745`
**Issue:** For every id in `active \ reclaimed`, `sweep` reads the *current* slot contents
(`read_slot`, `:726`), derives the key from them (`:728`) and calls `remove` with that key (`:734`).
The id was snapshotted from `active` earlier under the `active` lock, but the slot can have been
clunked and reused since: `clunk` removes from the tpht *before* it can take `active` (it is blocked
by `sweep`), so the slot can be zeroed and a concurrent `insert` can place a new, live key in the same
slot (first-free-slot allocation). `sweep` then computes the newcomer's key and removes it, and also
drops its id from `active` (`:738`) — a live fid silently disappears and its bitmap bookkeeping
desynchronises. When the slot reads as all zeros instead, the derived key is `(0, 0)` and
`remove(0, same)` targets the home block of key 0, i.e. an unrelated block.
**Fix:** remember the key alongside the slot id when the entry is inserted (e.g. a
`BTreeMap<(u8, u32), u64>` beside `active`, or store the key in a parallel slot map) and pass the
*recorded* key to `remove`; skip the id when `read_slot` returns zero words. Add a test that
clunks-and-reinserts the same slot while a sweep is in progress.

### WR-06: An out-of-window `ptr` in a handle aliases another slot; the bitmaps then name the wrong id

**File:** `port/src/fid.rs:129-140`, `:629-646`, `:693-701`; `ds/src/tpht/flattened.rs:488-515`
**Issue:** `StatefulHandle::from_wire` accepts any nonzero `ptr` in `1..=255` (`fid.rs:131`), and
`slot_index` masks the computed index with `slot_mask` (`flattened.rs:513-515`). For a handle whose
`ptr` is above the home window (17..=255) the masked slot can be a *different* slot of the shard —
documented for `deref_tiny` as "the full pointer range", but `clunk` (`:640`) and `mark_reclaimed`
(`:697`) then feed that aliased `slot_id` to `active.remove`/`reclaimed.insert` while `tpht.remove`
operated on the handle key's *home* block. A client that edits the `ptr` byte of its own handle (it is
an unauthenticated cleartext field in a packet the client seals) can therefore make the table drop a
live slot id from `active` or mark an unrelated slot reclaimed — the entry itself is not resolved
away, so this needs the aliased slot to hold an entry with the same `(client_id, fid_key)` (reachable
via the fallback-shard duplicate path, WR-09), but the bitmap state is then wrong and a subsequent
sweep acts on it.
**Fix:** validate the pointer against the home window in `resolve` — accept only `ptr.get() as usize
<= HOME_SLOTS` for a home-only shard — and use the resolved entry's own identity rather than
recomputing `slot_id` from the untrusted handle in `clunk`/`mark_reclaimed`.

### WR-07: Re-key control messages carry tag 0, not the pinned `NOTAG`

**File:** `port/src/chan/transport.rs:74`, `:1191-1196`
**Issue:** `REKEY_TAG` is `0`, with the comment "The tag the re-key control messages carry (P3)", but
DECISION P3 pins `Trekey`/`Rrekey` as "tag `NOTAG`, zero `FidHandle`", and `devmnt::NOTAG` is
`0xFFFF` (`port/src/devmnt.rs:116`). The exchange's `Tauth`/`Twrite`/`Tread` tags (1/2/3, `:69-71`)
also have no pinning decision and collide with ordinary tags (CR-02). No test asserts the control
packet's tag: `aead_transport_rekey_marker_then_ack` checks kind and generation, and the
independent-bytes test reads tag bytes only to re-frame exchange messages.
**Fix:** use `devmnt::NOTAG` for the control messages (or amend P3 in the same change and say why), and
add an assertion on the header tag byte (`packet[5..7]`) for a MARKER and an ACK.

### WR-08: `cargo xtask test-9p` cannot detect a listed test file that lost all its tests

**File:** `xtask/src/test9p.rs:150-192`
**Issue:** The port leg runs all five files as `--test aead_wire --test aead_tlv …` in one cargo
invocation, and `check_leg_output` returns `Ok` on the first `test result: ok. N passed` line with
`N > 0` (`:171-184`). If one listed file loses every test (or is emptied), the other four still print
positive lines and the leg passes. The module doc claims a filter that matches nothing cannot pass and
that a renamed/deleted file must fail loudly; `preflight` covers deletion, but nothing covers a file
whose tests were removed, which is the same "shrink into a silent pass" failure T-04-39 names.
**Fix:** run one leg per listed file (ten command specs, or five pairs), or parse every result line and
require at least `PORT_TESTS.len()` distinct positive `test result: ok` reports. Add a test feeding a
multi-target stdout with one zero-test line.

### WR-09: `insert` can create two live entries for one `(client_id, fid_key)` after a fallover

**File:** `port/src/fid.rs:551-592`, `:525-527`
**Issue:** `insert` starts at the key's home shard and walks forward on `HomeFull`, so a key whose
home block was full lands in shard *n+1*. A later `insert` of the same key starts at the home shard
again; if a slot there has since been clunked, `tpht.insert` finds room and reports `Ok`, because the
`same` duplicate check only sees the home shard's block (`:563-567`). The key now has two entries, a
handle to each, in two shards. `clunk`/`mark_reclaimed`/`resolve` all operate per handle, so the
first copy is leaked until it is clunked with its own handle or a sweep collects it while the second
copy remains live.
**Fix:** before the fallover insert, walk every shard from the home index and let each `tpht.insert`
report `Exists` for the key (i.e. start the duplicate probe at the home shard even when it is full,
which `ds::insert` already does for the *occupied* slots — the problem is only shards the walk
skipped), or record the fallback shard for the key and start the walk there.

## Info

### IN-01: Module docs describe the pre-implementation state

**File:** `port/src/chan/transport.rs:42-47`, `:879-885`; `port/src/chan.rs:27-31`; `port/src/lib.rs:88-94`
**Issue:** `transport.rs` still says "Re-keying is plan 04-08; its timing needs `now`, and this plan
threads that argument through without reading it" and "Plan 04-08's pending and superseded key slots
join this call; they do not exist yet" — while both are implemented in the same file. `chan.rs` says
TLVs, message bodies, the transport state machine and re-keying "are later plans", and `fid.rs`'s
declaration comment says "The table, the shards and the single-read resolve land in later plans".
**Fix:** update the four doc paragraphs to describe the shipped code; the next phase reads them as
fact.

### IN-02: `DataLayout::pages` is computed and never used; `crypt_body` recomputes it with a literal

**File:** `port/src/chan/msg.rs:480-506`; `port/src/chan/transport.rs:1448`, `:1467-1474`
**Issue:** `data_layout` returns `pages` and `crypt_body` ignores it, re-deriving the page walk from
`PAGE_BLOCKS` and incrementing its own `page` counter. Two places encode the same P5 arithmetic; a
change to one would not be caught.
**Fix:** have `crypt_body` drive its loop from `layout.pages` (or drop the field and document the
walk as the single source).

### IN-03: `wire::is_t_message` has no production caller

**File:** `port/src/chan/wire.rs:257-260`
**Issue:** The function is used only by its own pin (`aead_wire.rs:245-248`). It is a public API with
a test as its only consumer, which reads as a real capability check that nothing performs.
**Fix:** use it where a T-message is expected (e.g. `Session::seal` could reject an even/odd mismatch
against `Role`), or mark it `#[allow(dead_code)]` with a note that Phase 6 will use it.

### IN-04: `Msg`'s hand-written `PartialEq` ends in `_ => false`, which hides a new variant

**File:** `port/src/chan/msg.rs:201-470`
**Issue:** `Msg` derives `Clone, Debug` but implements `PartialEq` by hand with a `_ => false` arm
(`:467`). Adding a variant therefore compiles silently and the new variant compares unequal to
itself, which would break every round-trip pin at runtime rather than at compile time.
**Fix:** either derive `PartialEq` for `Msg` (all fields are comparable except `Qid`, which can gain a
manual `PartialEq` in `dat.rs`) or add `fn assert_exhaustive(a: &Msg, b: &Msg)` that enumerates every
pair, so a new variant is a compile error.

### IN-05: The threat-model overclaim guard only fires on rows carrying the Phase 4 status

**File:** `xtask/tests/threat_model.rs:228-238`, `:253-281`
**Issue:** `has_unnegated_use_claim` runs only on rows that contain `PHASE4_STATUS`. A row that says
"not implemented" *and* "used by a booted kernel" passes both filters, so the guard does not actually
prevent an overclaim; it only checks the six rows the author expected to tag. The negation test is
also the literal suffix `"not "`, so "not yet used by a booted kernel" would be reported as an
overclaim.
**Fix:** run `has_unnegated_use_claim` over every row (the phase column already tells you which rows
are Phase 4) and accept `not`, `never` and `not yet` before the phrase.

### IN-06: The end-to-end capability test claims "zero FID memory" but does not measure it

**File:** `port/tests/aead_negotiate.rs:582`, `:604-608`
**Issue:** The assertion compares `server.fid_memory()` (two bitmap lengths) before and after a
capability read. A `verify_cap` that touched a shard's arena or header would not change either
bitmap, so the test verifies "no bitmap mutation", not the WIRE-08 claim. The real proof is
`aead_fid_stateless_uses_zero_fid_memory` with `CountingProbe`; this test's message overstates.
**Fix:** reword the assertion, or give `E2eServer` a `CountingProbe` and assert `(arena_reads,
header_reads)` unchanged here too.

### IN-07: The `ds` removal test's "the freed slot takes another key" is not asserted

**File:** `ds/src/tpht/flattened.rs:1487-1535`
**Issue:** After removing the first key the test inserts a second key "into the same block" (the
comment on the insert at `:1520` says "the freed slot takes another key") and checks only that it
reads back correctly. With 16 free slots, `insert` may place the second key in any free slot, so the
test does not show that the *reused* slot (the one `deref_tiny` still points at) works.
**Fix:** assert `other_ptr == ptr` (the insert reuses the lowest free slot) before the read-back, so
slot reuse is actually covered.

---

_Reviewed: 2026-11-24_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
