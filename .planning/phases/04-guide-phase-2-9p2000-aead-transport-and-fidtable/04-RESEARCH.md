# Phase 4: Guide Phase 2 - 9P2000.aead Transport and FidTable - Research

**Researched:** 2026-11-24
**Domain:** Authenticated-encryption wire protocol codec and transport state machine in `no_std` Rust; TPHT-backed fid table; kernel dependency and byte-change procedure
**Confidence:** HIGH for what exists at HEAD and for the measured build results; MEDIUM for the design proposals in the discretion areas (they are proposals, tagged `[ASSUMED]`, and several fix one-way wire bytes)

Provenance tags: `[VERIFIED: path:lines]` means the file was opened with Read this session and the value is quoted beside the tag. `[MEASURED]` means a command was run this session and its output is quoted. `[CITED: url]` is an official document. `[ASSUMED]` is a proposal or training knowledge and needs confirmation before it becomes a locked decision.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried in (not re-asked)
- **D-01:** Keyed BLAKE3 is the only MAC; the packet MAC field is 32 bytes; "HMAC-BLAKE3" means BLAKE3 keyed mode with truncated output (Phase 3 D-05, D-07).
- **D-02:** ChaCha20 uses the RFC 8439 layout, 32-bit block counter and 96-bit nonce (Phase 3 D-06).
- **D-03:** Every requirement follows IG-V01: committed RED pin, implementation, GREEN twice, pin advance.
- **D-04:** Owner standing rules apply (no push without approval, no AI attribution in commits, Harvey read-only), except the `port/src` diff-first rule, which is changed for this phase by D-24. The compendium's appendix listings are non-normative sketches and are not copied as-is.
- **D-05:** TPHT keeps the 16-slot home block and the measured spill threshold from Phase 3; `FidTable` is built on `ds/src/tpht/` as it stands.

#### Wire header and AAD (W1 settled)
- **D-06:** The header is 24 bytes in the RFC section 4.1 order: Size(4) Type(1) Tag(2) Pad(1) FidHandle(8) SeqNonce(8) ExtMask(2) ExtTLVs Length(2), then TLVs, then ciphertext, then the 32-byte MAC. All multi-byte fields are little-endian. — **Reversibility:** one-way — this is the published wire contract of the dialect; any peer built against it breaks if it changes.
- **D-07:** The MAC covers every byte of the packet before the MAC, as one contiguous slice from byte 0 of Size to the last ciphertext byte. `Pad` and `ExtTLVs Length` are therefore authenticated. The receiver verifies before it acts on any field. The RFC section 4.3 AAD formula (and the same formula in guide section 6.3 and compendium section 5.3) is superseded. — **Reversibility:** one-way — the MAC input is part of the wire contract.
- **D-08 (rest of N1 settled):** The 12-byte ChaCha20 nonce is a 4-byte prefix followed by the 8-byte wire sequence nonce. The prefix is not sent on the wire: it is derived per connection and per direction from the `Tauth` key derivation, alongside the keys (the TLS 1.3 shape). — **Reversibility:** one-way — the nonce construction is part of the wire contract.
- **D-09:** In data messages (`Twrite`, `Rread`, `Tstream`) the encrypted body is two parts: a small parameter block, then file data whose length is a whole number of 4096-byte pages. Short data is padded and the true count is in the parameters. The receiver can read header, TLVs and parameters into a control buffer and the pages straight into page frames. There is no rule about the data's offset within the packet. — **Reversibility:** costly — the Phase 5 DMA ingress path is built on this body shape.
- **D-10:** The settled layout is recorded in three places: the RFC text is amended with dated notes (sections 3.2, 4.1 to 4.4, 5 as needed); PROJECT.md gets dated Key Decisions rows naming the superseded wording and W1, W2, W4 leave the Open Questions tables in PROJECT.md and STATE.md; and compendium listing 9.9 in `.planning/r9-architecture-and-code-v4.md` is rewritten in place with a dated note (unlike Phase 3, where the compendium was left unedited).

#### StatelessCap on the wire (W2 settled)
- **D-11:** The header `FidHandle` field stays 8 bytes in every packet. A `StatelessCap` travels in its own TLV under a new, sixth `ExtMask` bit (working name `EXT_STATELESS_CAP`). `EXT_CAP_TOKEN` keeps its single meaning, the 32-byte per-user token of RFC section 4.4. WIRE-02's "all five extension bits" and the RFC section 4.4 list are amended to six. — **Reversibility:** one-way — the extension bit and TLV are part of the wire contract.
- **D-12:** The new `ExtMask` bit is the discriminator between the two handle kinds. When it is set, the 8-byte header field must be all zero and the packet is rejected otherwise. A `StatefulHandle` can never be all zero because its `TinyPtr` byte is non-zero.
- **D-13:** The domain secret that signs and verifies `StatelessCap` tags is a caller-supplied `crypto::Key` parameter of the `FidTable` API. This phase proves the property in `test-9p`: two table instances given the same key verify each other's capabilities with zero FID memory. No kernel entropy source and no provisioning in this phase.
- **D-14:** The capability tag stays an 8-byte truncation of keyed BLAKE3 and `StatelessCap` stays 24 bytes, as WIRE-07 and every source state.

#### Session keys and Tauth (W4 settled; N5 for this phase)
- **D-15 (W4 settled):** Transient keys are per connection and per direction. One `Tauth` yields, for each direction, a cipher key, a MAC key and the nonce prefix of D-08. The FID is bound by the MAC over the header, not by a key. There is one sequence counter per direction. Keys are zeroized on connection teardown, not on `Tclunk`. The RFC section 3.2 per-FID formula and its `Tclunk` teardown rule are amended. — **Reversibility:** costly — the `crypto/src/session.rs` API, the nonce domain and re-keying are all built on this scope.
- **D-16 (N5 for this phase):** The session secret is injected behind a seam. `Tauth` / `Rauth` carry opaque exchange bytes, a trait hands the transport a 32-byte session secret, and tests supply the secret directly. The real authenticated exchange and the Noise versus DP9IK choice are not decided here. Stated limit: no live authenticated connection between two real machines comes out of this phase.
- **D-17:** `Tversion` / `Rversion` and the `Tauth` exchange travel in plain 9P2000 framing. Everything from `Tattach` onward uses the sealed envelope, with sequence counters starting at 0. A hash of the cleartext handshake bytes is mixed into the key derivation, so an altered version string or msize gives the two ends different keys and the first sealed packet fails. — **Reversibility:** one-way — the handshake transcript binding is part of the wire contract.
- **D-18:** Re-keying (WIRE-04) is a fresh `Tauth` exchange run in band, inside the sealed channel, deriving new keys from a new session secret. Under D-16 this calls the exchange seam again and tests supply the second secret. A hash ratchet was offered and declined.
- **D-19:** Either side starts a re-key when its send counter reaches a fixed threshold constant, well below 2^64 - 1 and settable low in tests. If the counter would reach the limit with no new keys, the sender closes the connection rather than wrap.
- **D-20:** Cutover is per direction, strict, with acknowledgement and timeout:
  1. The sender's last packet under its old send keys is a sealed "keys change now" marker; its next packet uses the new keys with the counter back at 0.
  2. The receiver, on the marker, switches its receive keys for that direction and sends a "key change complete" acknowledgement, which proves it derived the same keys. The receiver zeroizes its old keys for that direction on the marker.
  3. The sender zeroizes its old keys for that direction when the acknowledgement arrives.
  4. If the acknowledgement does not arrive within a fixed timeout, the sender drops the connection and zeroizes everything.
  5. An old-key packet is never accepted after the marker, with or without the acknowledgement. There is no overlap window. Outstanding requests and replies continue across the cutover.
- **D-21:** Sealing and sending happen under one per-direction lock, so packets leave in counter order. D-20's strictness relies on this and on the ordered, reliable byte stream 9P runs over.

#### Kernel reach and landing sites
- **D-22:** The code lands where the guide puts it: `port/src/chan.rs` (wire codec, `ExtMask` / TLVs, transport state), `port/src/fid.rs` (`FidHandle`, `FidEntry`, `FidTable`, `FidShard`), and `crypto/src/session.rs` (session key derivation). `port` gains its dependency on `crypto/` and `ds/` in this phase. No new workspace crate.
- **D-23:** Nothing in a booted kernel uses the new engine. `devmnt`'s `negotiate` gains the branch where an agreed `"9P2000.aead"` hands the session to the new engine; `devmnt` still offers plain `"9P2000"` unless asked. Host tests pin both branches against the real `devmnt` code, and the plain path is pinned unchanged (WIRE-12). The engine, session keys and `FidTable` are proven on the host by `test-9p`, with a client end and a server end over an in-memory link.
- **D-24:** The `port/src` diff-first stop is waived for this phase (owner: "the tests serve as inter-plan validation, so reserve code-review for phase end"). Plans commit once their tests are green; the owner reviews the whole `port/src` change at the phase-end code review, before any push. The standing rule is unchanged for other phases.
- **D-25:** The kernel byte change is accepted. The full x86_64 byte-change procedure runs as planned work in this phase: `boot-iso` `outputHash` re-derived and committed on the owner's x86_64 Linux host, the x64 TCG gates in an amd64 container, and the owner's linux boot-iso check. aarch64 gets the quick pass. "No kernel change" in this phase means no behaviour change; boot to rio is the invariant. Phase 2 D-23 to D-29 (end-of-phase stop, pushed branch as transport, owner makes the hash commit, order of steps) are the model; they were not re-asked.
- **D-26 (N11, wire half):** The packet MAC is verified under the connection's keys when the packet is received, before any field is acted on (WIRE-03). Whether Phase 5 adds a second, page-level check under page keys at fault time stays open for Phase 5; N11 is recorded as half-settled.

#### Not decided in discussion
- **IG-D11 lock:** W1, W2 and W4 are now settled, which was the stated condition for locking IG-D11 (Phase 2 D-07). The lock itself was not discussed. The plan must put it to the owner as a checkpoint; it must not be locked by assumption.

### Claude's Discretion
- The bit value and final name of the sixth `ExtMask` bit, and the TLV record format (type and length encoding, ordering, 64-bit alignment, treatment of unknown bits) within WIRE-02.
- The exact `info` and label layout of the session derivation (direction labels, how the handshake hash and nonce prefix are produced), provided it is pinned by a known-answer vector; the shape of the exchange seam trait.
- The re-key threshold constant, the acknowledgement timeout value and where the transport gets its clock; the opcodes or encodings of the marker and acknowledgement.
- What the `StatelessCap` tag covers beyond its own fields, and the meaning of its `epoch` field in this phase.
- Body encodings of the four new message pairs to the extent WIRE-06 needs round trips; the `FidShard` shape and bitmap representation for the sweep (WIRE-09).
- Layout of `cargo xtask test-9p`, where the host tests sit, module layout inside `chan.rs` / `fid.rs` (including split into submodules), and the order of work and commits.
- How `port`'s per-module `forbid(unsafe_code)` and `UNSAFE-AUDIT.md` are kept true for the new modules. A new unsafe-holding module in `port/` still needs owner approval (IG-D07).

### Deferred Ideas (OUT OF SCOPE)
- **Overlap window during re-key** (old and new keys both accepted for receipt): only if the dialect ever runs over an unordered or multi-path transport. The strict next-in-sequence rule would have to change first.
- **Real `Tauth` key exchange and the Noise versus DP9IK choice:** when a factotum exists to run it (Phase 12, or earlier if factotum is pulled forward).
- **Provisioning the `StatelessCap` domain secret to nodes:** Phase 6, with discovery and replicas.
- **A live sealed mount in a booted kernel:** the phase that has a real exchange and a server to talk to.
- **The second half of N11** (a page-level check at fault time): Phase 5.
</user_constraints>

## Contradictions and gaps the planner must put to the owner first

These were found while researching and are not resolved here. C1 to C3 touch one-way wire bytes, so they belong in one owner checkpoint **before the first wire pin (WIRE-01) is committed**, together with the IG-D11 lock.

| # | Finding | Evidence | Recommendation |
|---|---------|----------|----------------|
| **C1** | **D-06 says "24 bytes" but its own field list sums to 28.** Size 4 + Type 1 + Tag 2 + Pad 1 + FidHandle 8 + SeqNonce 8 + ExtMask 2 + ExtTLVs Length 2 = 28. The RFC diagram is seven 32-bit rows. The discussion log repeats the figure ("Whole 24-byte header", and "Widen the header handle to 24 bytes → 40-byte header", which is 24 + 16). | `[VERIFIED: .planning/rfc-9p2000-aead.md:102-136]` rows `Size (4B)`, `Type (1B) / Tag (2B) / Pad (1B)`, `FidHandle (8B)`, `Monotonic Sequence Nonce (8B)`, `ExtMask (2B) / ExtTLVs Length (2B)`; `[VERIFIED: 04-DISCUSSION-LOG.md:18,64]` | Treat the field list and order as the decision and the number as an arithmetic slip: `HEADER_LEN = 28`. The owner confirms in one line. Do not pad or drop a field to reach 24 without the owner. |
| **C2** | **The three volatility-class values collide with standard 9P `Qid.type` bits.** WIRE-10 puts `QTIMMUTABLE 0x20`, `QTLEASED 0x40`, `QTVOLATILE 0x80` in `qtype`. Plan 9 already assigns those bits, and r9 tests `QTDIR` on every walk. A directory would read as `QTVOLATILE`, and a volatile file would read as a directory. No W or N item covers this. | `[VERIFIED: ../harvey/sys/include/libc.h:591-593]` `#define QTDIR 0x80`, `#define QTAPPEND 0x40`, `#define QTEXCL 0x20`; `[VERIFIED: port/src/dat.rs:73-74]` `pub const QTDIR: u8 = 0x80;` `pub const QTFILE: u8 = 0x00;`; `[VERIFIED: .planning/rfc-9p2000-aead.md:382-391]` `Immutable = 0x20`, `Leased = 0x40`, `Volatile = 0x80` | In this phase build `VolatilityClass` as its own one-byte value (the `volatility_class` byte of `EXT_QID_VOLATILITY`, and the `qid_type` byte of `StatelessCap` / `FidEntry`), with the three values and the mutual-exclusion check. Do not add the constants to `port::dat` and do not OR them into the `dat::Qid.typ` that `devmnt` and `namec` read. Pin the collision in a test so it cannot be forgotten. The owner decides how a qid carries both a class and `QTDIR` (Phase 6 needs it for DIST-04). |
| **C3** | **The RFC opcode registry omits `Rerror` (107) and `Tflush`/`Rflush` (108/109).** WIRE-06 says "every base 9P2000 message". The base set has them, and a protocol with no error reply cannot run. | `[VERIFIED: ../harvey/sys/include/fcall.h:90-121]` `Terror = 106, /* illegal */`, `Rerror,`, `Tflush = 108,`, `Rflush,`; `[VERIFIED: .planning/rfc-9p2000-aead.md:290-307]` registry rows 100-105, 110-127, 130-137 only | Encode and decode the full `fcall.h` set (100 to 127, with 106 rejected as illegal) plus 130 to 137, and add 106 to 109 to the D-10 RFC amendment. |
| C4 | D-09 and WIRE-06 name `Tstream` as a data message. In the RFC formats `Tstream` has no data field and `Rstream` has `count[4] data[...]`. | `[VERIFIED: .planning/rfc-9p2000-aead.md:337-338]` `size[4] Tstream tag[2] fid_handle[8] stream_id[4] flags[2] offset[8]` and `size[4] Rstream tag[2] stream_id[4] chunk_seq[8] count[4] data[...]` | Apply the D-09 two-part body to every message that has a data field: `Twrite`, `Rread`, `Rstream` (and `Tstream` if Phase 6 gives it one). Record it in the RFC amendment. |
| C5 | D-16 says "`Tauth` / `Rauth` carry opaque exchange bytes". In 9P2000 `Tauth` is `afid[4] uname[s] aname[s]`, `Rauth` is `aqid[13]`, and the auth protocol's bytes travel as reads and writes on `afid`. | `[VERIFIED: ../harvey/sys/include/fcall.h:33-35]` `u32int afid; /* Tauth, Tattach */`, `char *uname;`, `char *aname;` | Keep the standard `Tauth` / `Rauth` bodies and carry the opaque exchange bytes in `Tread` / `Twrite` on `afid`, all in plain framing, all in the transcript. Confirm with the owner that this is what D-16 means. |
| C6 | D-25 says "x64 TCG gates in an amd64 container". In Phase 2 the TCG gate ran natively on the owner's amd64 host. | `[VERIFIED: 02-OWNER-STOP.md:193-198]` "On this native amd64 host, which is the approved venue (no Docker container is needed)" | Model the stop on Phase 2: TCG gate on the Linux host. Ask the owner only if a container is wanted this time. |
| C7 | Phase 2's stop rule "a `.text` difference is a stop" cannot be reused. Adding the dependency edge alone, with no use of either crate, already changes `.text` bytes. | `[MEASURED]` see "Kernel build and byte change" below | Replace the rule with the evidence set proposed there. |

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| WIRE-01 | One canonical header layout, implemented and round-trip tested; AC listing 9.9 corrected | D-06/D-07 layout (C1: 28 bytes); Pattern 1 (typestate envelope); D-10 document edits listed under "Documents this phase edits" |
| WIRE-02 | `ExtMask` and TLV parsing over 64-bit aligned records (amended to six bits by D-11) | TLV record proposal (Pattern 3) with sizes per extension; unknown-bit rule |
| WIRE-03 | MAC failure terminates the connection and zeroes channel state without parsing | Pattern 1 (`Unverified` → `Verified`), Pattern 2 (`fail()`), new constant-time incremental verify in `crypto/` (Finding F3) |
| WIRE-04 | Strict per-direction 64-bit sequence nonces; in-band re-key before 2^64 - 1 | Pattern 4 (re-key state machine), marker/ack encoding, sans-IO clock (Finding F4) |
| WIRE-05 | `Tauth` before `Tattach` yields per-connection per-direction keys, zeroized on teardown | Pattern 5 (session derivation, KAT method), seam trait, zeroize limits (Pitfall 6) |
| WIRE-06 | Round-trip every base message and the registry including 130-137; page-aligned data | C3 (full `fcall.h` set), body-encoding proposal (Pattern 6), data-part keystream counter rule |
| WIRE-07 | `StatefulHandle` 8 bytes, `StatelessCap` 24 bytes, `FidEntry` 64 bytes `align(64)` | Pattern 7 (layouts, safe word packing, const size assertions), D-11/D-12 TLV |
| WIRE-08 | `FidTable` / `FidShard` on TPHT; one memory read; `StatelessCap` zero FID memory | Pattern 7; counting `Probe` is public; stale-slot problem (Pitfall 2); home-only shards |
| WIRE-09 | Failover sweep is exactly `active \ reclaimed` in one and-not | `RoaringBitmap::difference` / `andnot` exist; bitmap id proposal (Pattern 7) |
| WIRE-10 | `Qid` 13 bytes, three volatility classes, `#G` path formula | C2 (bit collision); existing 13-byte codec in `devmnt`; path formula as a pure function |
| WIRE-11 | Replayed `Twrite` dropped; spliced payload fails the MAC | Pattern 2 receive order; two-connection splice test; cross-direction test |
| WIRE-12 | `Tversion "9P2000.aead"` selects the engine; plain path unchanged | Pattern 8 (`negotiate` branch with the old signature kept); 14 existing call sites |
| WIRE-13 | `cargo xtask test-9p` exists and runs the wire pins | "test-9p layout"; `xtask/src/miri.rs` as the model; lint-contract rule R4 must change (Finding F2) |
</phase_requirements>

## Summary

Everything the envelope needs except session derivation already exists in `crypto/` and `ds/`, but three small API gaps must be closed inside those crates, because `port` cannot reach key bytes: `crypto::Key` has no public accessor, `Blake3::new_keyed` takes raw key bytes, and the only constant-time verify is one-shot. Sealing, opening and MAC verification therefore have to live in `crypto/src/session.rs` behind a per-direction key type, and `port/src/chan.rs` holds only framing, parsing and the state machine. `FidTable` can be built on `FlattenedTphtShard` as it stands for insert and `deref_tiny`, but `remove` leaves the arena slot's words in place, so a clunked handle still resolves unless `ds` clears the slot or the table adds its own liveness check.

`port` depending on `crypto/` and `ds/` was built this session for both kernel targets (and checked for riscv64) in a throwaway clone. It builds with no source change and no new warning. With the primitives reachable from `devmnt::negotiate`, text grows by about 55 KB on aarch64 and about 272 KB on x86_64 (193 KB of that is three unselected BLAKE3 routines compiled as scalar code). No FP or SIMD instruction is added. The dependency edge alone, with no use, already rearranges `.text`, so the Phase 2 "`.text` identical" criterion does not apply; this phase's evidence has to be whole-ELF hashes, symbol-size totals, the FP/SIMD scan and the boot gates. `cargo xtask test` fails the moment the edge is added, because lint-contract rule R4 forbids it; that rule and its fixtures change in the same commit.

The timeout of D-20 should not read a clock. The transport is written sans-IO: every entry point takes `now` as a `u64` of milliseconds from the caller, as smoltcp's `poll(timestamp)` does and as `devcap`'s tick hook already does in `port`. Host tests pass a fake value; nothing in the kernel is wired this phase.

**Primary recommendation:** Open the phase with one owner checkpoint (C1, C2, C3, C5, the discretion proposals that fix wire bytes, and the IG-D11 lock), then build bottom-up: `crypto::session` → wire codec and TLVs → sealed transport and re-key → `fid` → `devmnt` branch and the dependency edge → `test-9p` → documents → phase-end code review → byte-change stop.

## Project Constraints

No `./CLAUDE.md` or `.claude/CLAUDE.md` exists in the repository `[MEASURED: ls]`. No project skills directory exists (`.claude/skills`, `.agents/skills`). The constraints that bind this phase come from the owner's global instructions and STATE.md:

- No AI attribution in any commit message or PR, ever (global rule; overrides harness defaults).
- No push without explicit owner approval; show what is unpushed first. Local `main` is 10 commits ahead of `origin/main` at research time `[MEASURED: git log origin/main..HEAD | wc -l]`.
- Harvey (`../harvey`) is read-only.
- The `port/src` diff-first rule is waived for this phase only (D-24): no per-plan owner diff stops. The owner reviews the whole `port/src` change at the phase-end code review, before any push.
- A new unsafe-holding module in `port/` needs owner approval (IG-D07). The recommendation below needs none.
- x86_64 kernel byte change: re-derive and commit `boot-iso` `outputHash`, TCG gates, the owner's Linux boot-iso check. aarch64: quick pass.
- Kernels are built without FP/SIMD.
- Build and test: `nix develop -c cargo xtask check`, `nix develop -c cargo xtask test`.
- macOS Unix socket paths cap at 104 bytes: run boot tests from a short path.

## Architectural Responsibility Map

r9 is a kernel, so the tiers are crates, not web tiers.

| Capability | Primary owner | Secondary | Rationale |
|------------|---------------|-----------|-----------|
| Session key derivation, per-direction keys, nonce prefix | `crypto/src/session.rs` | — | Key bytes are reachable only inside `crypto/` (`Key::bytes` is `pub(crate)`) |
| Packet MAC compute and constant-time verify; body keystream | `crypto/src/session.rs` | `crypto/src/mac.rs`, `page.rs` | Same reason; `port` must never hold raw key bytes |
| `StatelessCap` tag compute and verify | `crypto/` (a small function taking `&Key`) | `port/src/fid.rs` calls it | `verify_keyed` takes raw key bytes, which `port` cannot supply from a `Key` |
| Header, `ExtMask`, TLV, message body encode and decode | `port/src/chan.rs` (or `chan/` submodules) | — | D-22 landing site; pure byte work, `forbid(unsafe_code)` |
| Transport state: sequence counters, strict receive, re-key, teardown | `port/src/chan.rs` | `crypto::session` | D-22; sans-IO, driven by the caller |
| `FidHandle`, `FidEntry`, `FidShard`, `FidTable`, sweep | `port/src/fid.rs` | `ds::tpht`, `ds::roaring` | D-22 |
| Clearing a removed slot; slot id helper | `ds/src/tpht/flattened.rs` | — | Only `ds` can write an arena slot |
| Version selection point | `port/src/devmnt.rs` (`negotiate`) | `chan` | D-23 |
| `test-9p` step; lint-contract rule change | `xtask/` | — | No approval needed for `xtask/` edits |
| Server end of the protocol | host tests only (`port/tests/`) | — | r9 has no in-kernel 9P server (CONTEXT) |

## Standard Stack

No new external crate. Everything is in the workspace already.

### Core
| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| `crypto` (workspace) | 0.1.0 | HKDF, keyed BLAKE3, ChaCha20, `Key` | Phase 3 output; vector-pinned on the default and scalar backends |
| `ds` (workspace) | 0.1.0 | `FlattenedTphtShard`, `TinyPtr`, `RoaringBitmap` | Phase 3 output; D-05 |
| `blake3` | `=1.8.7` | behind `crypto::mac` | pinned in Phase 3 `[VERIFIED: crypto/Cargo.toml]` |
| `chacha20` | `=0.10.2` | behind `crypto::page` | pinned in Phase 3 |
| `hkdf` / `hmac` / `sha2` | `=0.13.0` / `=0.13.0` / `=0.11.1` | behind `crypto::kdf`, `crypto::key` | pinned in Phase 3 |
| `zeroize` | `=1.9.2` | `Key` storage | pinned in Phase 3 |

### Alternatives Considered
| Instead of | Could use | Tradeoff |
|------------|-----------|----------|
| Sealing inside `crypto::session` | A public `Key::expose()` so `port` drives `Blake3` and `chacha20_xor` itself | Breaks Phase 3's "no public function returns key bytes" (T-03-14). Do not. |
| `mcslock::Lock<RoaringBitmap>` in `FidShard` | `spin::RwLock` as in compendium listing 9.6 | A new crate for nothing; `port` already has `Lock`. Do not add `spin`. |
| Hand-written header struct with `from_le_bytes` | `zerocopy` derive (already a dependency of `x86_64`, not of `port`) | A new `port` dependency and a `repr(packed)` struct for 28 bytes. Not worth it. |

**Installation:** none. The one manifest change is in `port/Cargo.toml`:

```toml
crypto = { path = "../crypto" }
ds = { path = "../ds" }
```

`Cargo.lock` changes by two lines only (the `port` package gains `"crypto"` and `"ds"` in its dependency list) `[MEASURED: git diff Cargo.lock in the probe clone]`.

## Package Legitimacy Audit

No package is installed in this phase, so the legitimacy gate has nothing to check. The six `crypto` pins and the `roaring` test oracle were audited in Phase 3 (`03-RESEARCH.md`, all `OK`).

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

One optional change touches a pinned crate's feature list, not the package set: enabling `blake3`'s `zeroize` feature (see Pitfall 6). It adds no crate; `zeroize` is already locked. It is an owner choice, not a recommendation to do silently.

## Findings

### F1. Kernel build and byte change `[MEASURED]`

Procedure: a `git clone` of HEAD `6263543` into the session scratch directory (not a worktree; the working tree was never edited and `git status` is unchanged), three stages, each built with `nix develop -c cargo xtask build --arch aarch64 --config virtshell` and `--arch x86-64 --config q35churnshell`, venue `Darwin arm64`. The clone was deleted afterwards.

- **before:** HEAD as is.
- **depsonly:** the two lines added to `port/Cargo.toml`, nothing else.
- **used:** a probe function reachable from `devmnt::negotiate` (in an `if version == b"9P2000.aead"` branch) that calls `derive_labelled`, keyed `Blake3`, `chacha20_xor`, `hkdf_extract`, `verify_keyed`, `FlattenedTphtShard::<8>::with_banks(64, 0)`, `insert`, `deref_tiny` and `RoaringBitmap::andnot`.

All six builds exit 0. No new warning appears in either `used` build log compared with `before`.

| Stage | Config | ELF sha256 (first 16) | ELF bytes | Defined text symbol bytes | Stripped ELF bytes differing from `before` |
|-------|--------|-----------------------|-----------|---------------------------|--------------------------------------------|
| before | `virtshell` | `9218b998327c49b6` | 63,267,544 | 754,232 | — |
| depsonly | `virtshell` | `37453eb237082c03` | 63,267,616 | 754,232 | 1,165,403 |
| used | `virtshell` | `680ef25e6370b781` | 66,009,712 | 808,908 | 1,240,900 |
| before | `q35churnshell` | `c9dcef0b32b77e13` | 28,062,744 | 759,072 | — |
| depsonly | `q35churnshell` | `f816631cd5743154` | 28,062,624 | 759,072 | 1,206,264 |
| used | `q35churnshell` | `0ed7b324bdcb700c` | 31,362,368 | 1,031,479 | 1,512,534 |

What this shows:

1. **It builds in portable form for both kernel targets.** `nix develop -c cargo xtask check` (all legs, including `check --package riscv64 --bins`) and `nix develop -c cargo xtask build --arch riscv64` also exit 0 with the edge added.
2. **The `before` hashes equal the Phase 3 baseline** (`9218b998…fa083e`, `c9dcef0b…8b78b`) although the clone sits at a different path, so on one venue the build is path-independent. The Phase 4 "before" record can therefore be taken at the phase's base commit on the Mac exactly as Phase 3 did.
3. **The edge alone changes `.text` contents** (717,615 differing bytes in the extracted `.text` on aarch64, 731,681 on x86_64) while the total of defined text symbol sizes is unchanged. The likely cause is that `port`'s crate hash changes with its dependency set, which changes mangled names and link order `[ASSUMED: cause not investigated]`. Consequence: Phase 2's ".text identical" verdict is unusable here (C7).
4. **Cost of the primitives when reachable:** +54,676 bytes of text on aarch64, +272,407 on x86_64. On x86_64, 271,799 bytes are in 95 `crypto`/`ds`/upstream symbols, led by `blake3::avx2::hash8` (105,207), `blake3::sse41::hash4` (43,994) and `blake3::sse2::hash4` (43,994). These are the intrinsics routines compiled as scalar code for the soft-float target; `blake3`'s `build.rs` emits `blake3_sse2_rust` / `blake3_avx2_rust` under `pure`, and the `no_*` features only make the run-time selector return false `[VERIFIED: blake3-1.8.7/build.rs:197-203, src/platform.rs:434,468 in the vendored registry source]`. They are never selected. The engine's own code comes on top of these numbers.
5. **No FP or SIMD instruction is added.** A disassembly scan for `%xmm`/`%ymm`/`%zmm`/`%st(`/`%mm` on x86_64 counts 6 lines before and 6 after; the aarch64 scan for vector and FP register operands counts 309 before and 309 after. (The pattern is crude, which is why the count is compared and not read as an absolute.)
6. **Section sizes do not move.** x86_64 `.text` is one 2 MiB unit before and after (`. = ALIGN(2097152)` in `x86_64/lib/kernel.ld` `[VERIFIED: x86_64/lib/kernel.ld:16-23]`). Text symbols use about 1.03 MB of it after the primitives, so about 1 MB of headroom remains before `.text` would grow to 4 MiB and move every later section. aarch64 `.data` grew by one page (1,978,368 → 1,982,464).
7. **`cargo xtask test` fails with the edge** until the lint contract changes: `lint-contract: 2 problem(s)` `[MEASURED]`. See F2.

**Evidence set to plan for this phase (replaces "`.text` identical"):** whole-ELF sha256 before and after on one venue for `virtshell` and `q35churnshell` (expected: different); defined text symbol byte totals before and after; the FP/SIMD scan counts equal to baseline; x86_64 text symbol total under 2,097,152; then the gates. A stop condition worth keeping: a non-zero change in the FP/SIMD scan counts.

### F2. The lint contract forbids the edge today

`[VERIFIED: xtask/src/lintcontract.rs:335-378]` rule R4 iterates `["port", "aarch64", "x86_64", "riscv64"]` and reports `kernel crate dependency ... no kernel crate may depend on crypto/ or ds/ in Phase 3 (D-21). Phases 4 and 5 add this dependency under the kernel byte-change procedure and must change this rule then`. `TestStep::run` calls it before any leg `[VERIFIED: xtask/src/main.rs:3570-3577]`.

Change R4 to: `port` may depend on `crypto` and `ds` by path; the arch crates (`aarch64`, `x86_64`, `riscv64`) still may not name them directly (they reach them through `port`). Update the module header, the message text and the fixture tests in the same commit as the manifest edge, and add a fixture proving an arch crate naming `crypto` is still rejected. R5 (no soft-backend cfg in `.cargo/config.toml`, no `miri` in `rust-toolchain.toml`) stays as is.

### F3. Feeding keyed BLAKE3 without copying, and the three gaps in `crypto/`

`[VERIFIED: crypto/src/mac.rs:45-71]`:

```rust
/// An incremental BLAKE3 hasher: unkeyed via [`Blake3::new`], keyed via
/// [`Blake3::new_keyed`].  Feeding the same bytes through any split of
/// `update` calls gives the one-shot result.
pub fn new_keyed(key: &[u8; BLAKE3_KEY_LEN]) -> Blake3
pub fn update(&mut self, data: &[u8])
pub fn finalize(&self) -> [u8; BLAKE3_OUT_LEN]
```

`update` borrows its input and copies nothing into a caller-visible buffer; upstream documents "Add input bytes to the hash state. You can call this any number of times." `[VERIFIED: blake3-1.8.7/src/lib.rs:1197-1206 in the vendored registry source]`. So header, TLVs and ciphertext can be three `update` calls over three borrowed slices, and in Phase 5 one `update` per page frame. When the packet is one contiguous buffer, a single `update(&packet[..size - 32])` is the D-07 slice.

Gaps, all inside `crypto/`:

1. **`port` cannot key the hasher.** `Key::bytes` is `pub(crate)` `[VERIFIED: crypto/src/key.rs:76-80]` and `Blake3::new_keyed` wants `&[u8; 32]`. The keyed hasher must be created inside `crypto`.
2. **There is no constant-time incremental verify.** `verify_keyed` is one-shot over a single `msg` slice `[VERIFIED: crypto/src/mac.rs:81]`. Add a `verify(self, tag: &[u8; 32]) -> crypto::Result<()>` on the packet-MAC type that compares in constant time (the accumulator fold already used at `mac.rs:101-108`).
3. **`port` cannot apply the keystream.** `chacha20_xor` wants `&[u8; 32]` too. Add a method on the per-direction key type.

The same applies to the `StatelessCap` tag: add `crypto` functions taking `&Key` (for example `cap_tag(&Key, &[u8]) -> [u8; 8]` and `cap_verify(&Key, &[u8], &[u8; 8]) -> Result<()>`), since `verify_keyed` cannot be called from `port` with a `Key`.

### F4. A sound source for the D-20 timeout

Do not read a clock in the transport. Make it sans-IO: `seal`, `open` and a `poll(now)` take `now: u64` (milliseconds, monotonic, supplied by the caller). The state machine stores `ack_deadline = now + REKEY_ACK_TIMEOUT_MS` when it sends the marker and fails closed when a later call sees `now >= ack_deadline`.

- This is the smoltcp shape `port` already lives with: `Interface::poll` takes a timestamp from the caller, and `devip` passes one in `[ASSUMED: from the smoltcp API as used in port/src/devip; not re-read this session]`.
- `port` already has the hook idiom for the day a kernel caller exists: `[VERIFIED: port/src/devcap.rs:109-123]` `static TICK_HOOK: Lock<Option<fn() -> u64>> = Lock::new("captick", None);` with `pub fn set_tick_hook(hook: fn() -> u64)`.
- Under D-23 nothing in the kernel drives the engine, so no hook is wired this phase. Host tests pass a fake `now` and step it.
- A deadline only fires when something calls in. State this in the module documentation: whoever owns the connection must call `poll(now)` periodically. That is Phase 6's concern when a live mount exists.

### F5. `devmnt` at HEAD

- `[VERIFIED: port/src/devmnt.rs:124]` `pub const VERSION9P: &[u8] = b"9P2000";`
- `[VERIFIED: port/src/devmnt.rs:93-113]` the client codec knows `TVERSION = 100`, `RVERSION = 101`, `TATTACH = 104`, `RATTACH = 105`, `RERROR = 107`, `TWALK = 110`, `RWALK = 111`, `TOPEN = 112`, `ROPEN = 113`, `TREAD = 116`, `RREAD = 117`, `TWRITE = 118`, `RWRITE = 119`, `TCREATE = 114`, `RCREATE = 115`, `TCLUNK = 120`, `RCLUNK = 121`, `TREMOVE = 122`, `RREMOVE = 123`, `TSTAT = 124`, `RSTAT = 125`. It has no `Tauth`, `Tflush` or `Twstat`. These constants are private to `devmnt`.
- `Mnt::negotiate(io_r: Box<dyn MntIo>, io_w: Box<dyn MntIo>, offer: u32) -> Result<Mnt>` sends `VERSION9P` and errors unless the reply equals it (`port/src/devmnt.rs:670-716`, read through the shell this session). It has one kernel caller (`devmnt.rs:1022`, the `fversion` path) and 14 call sites in `port/tests/devmnt_test.rs`.
- The transport seam is `MntIo` (`write_msg`, `read_some`, `close_chan`), with `FakeServer` / `FakeHalf` in the tests implementing it over an in-memory queue.
- `Qid` is `path: u64, vers: u32, typ: u8` `[VERIFIED: port/src/dat.rs:405-410]`; the 13-byte wire order `type[1] vers[4] path[8]` is in `RepCur::qid` (`devmnt.rs:205-210`).

### F6. `ds` at HEAD, as `FidTable` needs it

- `[VERIFIED: ds/src/tpht/mod.rs:82-97]` `pub struct TinyPtr(core::num::NonZeroU8);` with `new(value: u8) -> Option<TinyPtr>` and `get(self) -> u8`. D-12 rests on this.
- `[VERIFIED: ds/src/tpht/mod.rs:30,47]` `pub const HOME_SLOTS: usize = 16;` `pub const SPILL_THRESHOLD: usize = 16;`
- `[VERIFIED: ds/src/tpht/mod.rs:66-80]` `TphtError` is `BadBlockCount`, `Exists`, `HomeFull`, `BanksFull`.
- `insert(&self, key: u64, value: [u64; WORDS], same: impl Fn(&[u64; WORDS]) -> bool) -> Result<Option<TinyPtr>, TphtError>` returns `Ok(None)` for an entry that went to the overflow banks; such an entry "has no tiny pointer" (`flattened.rs:240-255`). **A spilled fid cannot be a `StatefulHandle`.**
- `[VERIFIED: ds/src/tpht/flattened.rs:159-167]` `with_banks(num_blocks, 0)` "builds a home-only shard: it holds no overflow and applies no spill threshold, taking entries until all 16 header slots of a block are used and then returning `TphtError::HomeFull`".
- `[VERIFIED: ds/src/tpht/flattened.rs:425-446]` `deref_tiny(&self, key: u64, ptr: TinyPtr) -> [u64; WORDS]`: "A stale pair returns whatever the slot now holds". `remove` clears the header fields and never writes the arena slot (`flattened.rs:382-415`).
- `Probe` (`header_read`, `arena_read`) and `with_banks_and_probe` are public `[VERIFIED: ds/src/tpht/mod.rs:52-58]`, so a `port` test can count reads.
- `[VERIFIED: ds/src/tpht/feistel.rs:67]` `pub fn split(key: u64, index_bits: u32) -> (usize, u64)` is public; the shard's own `split_key` and `slot_index` are private.
- `[VERIFIED: ds/src/roaring.rs:181-189]` `pub fn difference(&self, other: &Self) -> Self` and `pub fn andnot(&mut self, other: &Self)`; `RoaringBitmap` derives `Clone` and holds `u32` values.

## Architecture Patterns

### System Architecture Diagram

```
   host test: client end                                   host test: server end
        |                                                         ^
        v                                                         |
  [ plain 9P2000 framing ]  Tversion/Rversion, Tauth/Rauth, afid reads/writes
        |    every plain byte, both directions, in wire order --> transcript hash
        v
  [ SessionExchange seam ] --(32-byte secret, tests inject)--> crypto::session::derive
        |                                                         |
        |                       per direction: k_enc, k_mac, nonce prefix (4)
        v
  SEND (one lock per direction, D-21)            RECEIVE (single reader)
  encode body --> TLVs --> header(seq=n)         read Size (bounds vs msize only)
        |                                              |
        v                                              v
  keystream(prefix||seq) over body               read Size bytes
        |                                              |
        v                                              v
  keyed BLAKE3 over [0 .. size-32)               keyed BLAKE3 verify over [0 .. size-32)
        |                                          | fail --> zeroize both directions, CLOSED
        v                                          v ok
  append MAC, write whole packet                 seq == expected ?  no --> zeroize, CLOSED
        |                                          | yes
        |  n+1 == rekey threshold?                 v
        |     yes --> in-band Tauth,             rekey marker?  yes --> swap recv keys, send ack
        |             marker, new keys, n=0        | no
        v                                          v
   in-memory link  ------------------------->    decrypt body --> parse TLVs --> decode message
                                                       |
                                                       v
                                        FidHandle: ExtMask bit set?
                                          yes --> header handle must be zero --> StatelessCap TLV
                                                  --> cap_verify(domain key)   (no table memory)
                                          no  --> StatefulHandle --> FidShard[shard_id]
                                                  --> deref_tiny(key, ptr)     (one arena read)
                                                  --> validate fid_key, client_id, epoch, live

   devmnt::negotiate --(offer "9P2000", default)--> plain Mnt (unchanged path, boots to rio)
                     --(offer "9P2000.aead", asked)--> hand io halves + transcript to the engine
```

### Recommended Project Structure

```
crypto/src/
├── session.rs        # NEW: derive, DirectionKeys, PacketMac, cap tag helpers, zeroize
crypto/tests/
├── session.rs        # NEW: derivation KAT, direction separation, transcript sensitivity
port/src/
├── chan.rs           # NEW: module root: constants, re-exports, #[forbid(unsafe_code)] in lib.rs
├── chan/
│   ├── wire.rs       # header, opcode registry, Unverified/Verified packet types
│   ├── tlv.rs        # ExtMask, TLV records
│   ├── msg.rs        # message body encode/decode (base set + 130-137)
│   └── transport.rs  # per-direction state, strict receive, re-key, teardown, SessionExchange
├── fid.rs            # NEW: StatefulHandle, StatelessCap, FidEntry, FidShard, FidTable, VolatilityClass
├── devmnt.rs         # EDIT: negotiate branch only
├── lib.rs            # EDIT: two `#[forbid(unsafe_code)] pub mod` lines
port/tests/
├── common/aead_link.rs   # in-memory duplex link, client and server ends, fake clock
├── aead_wire.rs      # WIRE-01, WIRE-06 round trips, registry
├── aead_tlv.rs       # WIRE-02
├── aead_transport.rs # WIRE-03, WIRE-04, WIRE-05, WIRE-11
├── aead_fid.rs       # WIRE-07, WIRE-08, WIRE-09, WIRE-10
├── aead_negotiate.rs # WIRE-12 (against real devmnt code)
xtask/src/
├── test9p.rs         # NEW: `cargo xtask test-9p`
├── lintcontract.rs   # EDIT: R4
├── main.rs           # EDIT: subcommand registration
```

A forbid on `chan` in `port/src/lib.rs` reaches its submodules, which is what is wanted here (`port/src/lib.rs` explains this in its `devip_waker` comment).

### Pattern 1: Parsing is only possible on a verified packet (WIRE-03)

**What:** Two types. `Unverified<'a>` wraps the raw bytes and exposes nothing but `len()`. The only way to get a `Verified<'a>` is the transport's `open`, which runs the MAC. Every decode function takes `Verified`. "Parse before verify" then does not compile.

**The one field read before verification is `Size`,** and only to frame: it must be at least `HEADER_LEN + MAC_LEN` and at most the negotiated msize, as `frame_one` does today for plain 9P. Nothing else is read. In particular `ExtTLVs Length` is not needed to locate the MAC input: the MAC covers `[0, size - 32)` whatever the TLV length says.

```rust
// Sketch [ASSUMED design]; constants per D-06 with C1's reading.
pub const HEADER_LEN: usize = 28;
pub const MAC_LEN: usize = 32;

pub struct Unverified<'a>(&'a mut [u8]);          // whole packet, Size already bounds-checked
pub struct Verified<'a> { hdr: Header, tlvs: &'a [u8], body: &'a mut [u8] }

impl RecvState {
    /// Verify, then check the sequence number, then decrypt.  Any failure
    /// zeroizes the whole connection state and leaves it closed.
    pub fn open<'a>(&mut self, pkt: Unverified<'a>, now: u64) -> Result<Verified<'a>, Closed>;
}
```

### Pattern 2: Receive order and failing closed

1. Frame by `Size` (bounds only).
2. `PacketMac::verify` over `[0, size - 32)` against the last 32 bytes. Failure: `fail()`.
3. Read `SeqNonce`. It must equal the expected next value. Otherwise: `fail()`.
4. Check `Pad == 0`, unknown `ExtMask` bits clear, `ExtTLVs Length` consistent (zero if and only if the mask is zero; multiple of 8; not past the body). Otherwise: `fail()`.
5. Decrypt the body, hand `Verified` to the decoder.

`fail()` replaces both directions' keys and prefixes with zeroized values, zeroes the counters, sets `Closed`, and every later `seal`/`open` returns `Err(Closed)`.

For step 3, WIRE-11 and success criterion 2 say a replayed packet "is dropped". On an ordered, reliable stream (D-21) an unexpected sequence number means an attacker or a bug, and an injected packet has already desynchronised the stream, so the recommendation is to drop the packet **and** close the connection `[ASSUMED: owner may prefer drop-only; see A6]`.

### Pattern 3: `ExtMask` and TLV records (discretion proposal) `[ASSUMED design]`

```rust
pub const EXT_DEADLINE: u16       = 1 << 0;   // value 8 bytes
pub const EXT_CAP_TOKEN: u16      = 1 << 1;   // value 32 bytes
pub const EXT_QID_VOLATILITY: u16 = 1 << 2;   // value 10 bytes: class[1] lease_gen[8] granted_mode[1]
pub const EXT_STREAM_PRIO: u16    = 1 << 3;   // value 4 bytes: (tier << 30) | credit
pub const EXT_OBINARY: u16        = 1 << 4;   // value 0 bytes
pub const EXT_STATELESS_CAP: u16  = 1 << 5;   // value 24 bytes (D-11)
pub const EXT_KNOWN: u16          = 0x003F;
```

The first five values are the RFC's `[VERIFIED: .planning/rfc-9p2000-aead.md:174-178]` `EXT_DEADLINE = 1 << 0`, `EXT_CAP_TOKEN = 1 << 1`, `EXT_QID_VOLATILITY = 1 << 2`, `EXT_STREAM_PRIO = 1 << 3`, `EXT_OBINARY = 1 << 4`. `1 << 5` and the name are the proposal.

Record format:

| Bytes | Field |
|-------|-------|
| 1 | `type`: the bit index, 0 to 5 |
| 1 | reserved, must be 0 |
| 2 | `len`: value length in bytes, little-endian, unpadded |
| `len` | value |
| 0 to 7 | zero padding so the record is a multiple of 8 bytes |

Rules: records appear in ascending `type` order; each type at most once; the set of types equals the set bits of `ExtMask` exactly; `len` must equal the fixed length for its type; padding bytes must be zero; `ExtTLVs Length` is the sum of the padded records and is zero exactly when `ExtMask` is zero. Any bit outside `EXT_KNOWN`, or any rule broken, is a protocol error found after MAC verification and closes the connection. Both ends agreed the exact dialect string, so there is nothing to skip gracefully.

Resulting record sizes: deadline 16, cap token 40, volatility 16, stream prio 8, obinary 8, stateless cap 32.

"64-bit aligned" then means each record is a multiple of 8 bytes measured from the start of the TLV area. With a 28-byte header the TLV area starts at packet offset 28, which is not a multiple of 8. That does not matter: the parser borrows the bytes and reads fields with `from_le_bytes` (zero-copy in the sense of no allocation and no intermediate buffer, and required anyway under `forbid(unsafe_code)`). If the owner wants the TLV area 8-aligned from byte 0 of the packet, that is a header change and belongs with C1.

`EXT_STREAM_PRIO` uses the RFC's 2 + 30 split (ingest note I9 records the compendium's "32-bit stream credit" as read compatible).

### Pattern 4: Re-key (D-18 to D-20), marker and acknowledgement (discretion proposal) `[ASSUMED design]`

Constants, all fields of a `TransportConfig` so tests can set them low:

| Constant | Proposed default | Reason |
|----------|------------------|--------|
| `REKEY_AFTER_PACKETS` | `1 << 48` | "Well below 2^64 - 1" (D-19). At a million packets a second this is about nine years. |
| `SEQ_HARD_LIMIT` | `u64::MAX` | The sender never sends sequence `u64::MAX`; if the next value would be it and no new keys are installed, close (D-19). |
| `REKEY_ACK_TIMEOUT_MS` | `5_000` | The only timeout the sources give for a peer that must answer is the 5-second lease revocation default (RFC section 11 item 4). |

One new opcode pair, added to the registry (and to the D-10 RFC amendment): `Trekey = 138` when sent by the client, `Rrekey = 139` when sent by the server. Both use tag `NOTAG` and a zero `FidHandle`, and share one body:

```
kind[1] generation[4] confirm[32]
kind: 0 = REQUEST (please start the in-band Tauth), 1 = MARKER (keys change now), 2 = ACK
```

Flow for one direction (say client to server), generation `g` → `g + 1`:

1. Either side's send counter reaches `REKEY_AFTER_PACKETS`. The client starts a fresh `Tauth` exchange inside the sealed channel; a server that reaches the threshold first sends `REQUEST`.
2. The exchange seam yields a new 32-byte secret at both ends. Both derive generation `g + 1` keys for both directions and hold them as pending.
3. The sender of a direction sends `MARKER(g + 1)` as its last packet under the old keys of that direction, then switches its send keys and sets its send counter to 0. It stores `ack_deadline = now + REKEY_ACK_TIMEOUT_MS`.
4. The receiver, on `MARKER`, switches its receive keys for that direction, sets the expected counter to 0, zeroizes the old receive keys, and sends `ACK(g + 1, confirm)` in the other direction, where `confirm = keyed BLAKE3(new k_mac of the direction being cut over, "r9 rekey ack" || generation)`. Sealing the ack under the other direction's keys proves nothing about the new keys; the `confirm` value does. This is what "proves it derived the same keys" needs.
5. The sender verifies `confirm` in constant time and zeroizes its old keys for that direction. A wrong `confirm`, or `now >= ack_deadline` with no ack, is `fail()`.
6. Each direction cuts over independently. Requests and replies outstanding at the cutover are untouched: tags and fids are not keyed.

TLS 1.3 has the same per-direction shape without the acknowledgement: after sending KeyUpdate "the sender SHALL send all its traffic using the next generation of keys", and on wrap an implementation "MUST either rekey ... or terminate the connection" `[CITED: https://www.rfc-editor.org/rfc/rfc8446.txt sections 4.6.3 and 5.3]`.

### Pattern 5: Session derivation and its known-answer vector (discretion proposal) `[ASSUMED design]`

Inputs: `secret` (32 bytes from the seam), `transcript` (32 bytes), `generation: u32`.

```
transcript_0 = BLAKE3( every plain-framed handshake message, whole, in wire order )
               (Tversion, Rversion, Tauth, Rauth, each afid Tread/Rread/Twrite/Rwrite)
transcript_g+1 = BLAKE3( transcript_g || the plaintext of the in-band re-key exchange messages )

prk = HKDF-Extract(salt = transcript_g, ikm = secret)

info = "9P2000.aead" (11 bytes) || 0x00 || dir (1) || purpose (1) || generation (4, LE)     = 18 bytes
       dir:     0x01 client-to-server, 0x02 server-to-client
       purpose: 0x01 cipher key (32), 0x02 MAC key (32), 0x03 nonce prefix (4)

out = HKDF-Expand(prk, info, length)
nonce(seq) = prefix (4) || seq as 8 bytes little-endian                                    (D-08)
```

Why these choices:

- Every 9P message starts with its own `size[4]`, so concatenating whole messages is unambiguous. The handshake must be strictly serial (one request outstanding) so both ends see the same order.
- `info` is fixed-length. `derive_labelled` concatenates label and context unframed and its documentation tells callers to use fixed-length labels `[VERIFIED: crypto/src/key.rs:105-109]`.
- `generation` is in `info` so that a seam that returns the same secret twice still cannot reproduce a (key, nonce) pair after the counter returns to 0. Without it that would be keystream reuse.
- The prefix is derived, never sent, as D-08 requires. TLS 1.3 derives its per-direction IV the same way, `[sender]_write_iv = HKDF-Expand-Label(Secret, "iv", "", iv_length)` `[CITED: https://www.rfc-editor.org/rfc/rfc8446.txt section 7.3]`; WireGuard's nonce is "32 bits of zeros followed by the 64-bit little-endian value of counter" `[CITED: https://www.wireguard.com/protocol/]`. D-08 is the first shape with the second's byte order.
- BLAKE3 for the transcript needs no new primitive; `Blake3::new()` is already incremental. The alternative is SHA-256 (the TLS choice), which would need a new public `sha256` in `crypto` and a FIPS vector pin.

Existing API covers it: `hkdf_extract(salt, ikm) -> [u8; 32]`, `hkdf_expand(prk, info, okm)` (`crypto/src/kdf.rs`). `session.rs` wraps the outputs in `Key` and a private `[u8; 4]`.

**Known-answer vector method** (Phase 3's `k_file` precedent: "pinned by an independently computed Python hmac/hashlib KAT"): fix `secret`, `transcript` (as 32 given bytes) and `generation`; compute the six outputs with Python's standard `hmac` and `hashlib` (HKDF-SHA256 is ten lines); commit the script's output as constants in `crypto/tests/session.rs` with the script in a comment. Python 3.13 is on the host and in the devshell `[MEASURED]`. Python's standard library has no BLAKE3, so the KAT takes the transcript hash as an input, and a second test pins `transcript = blake3_hash(concatenation)` against the crate's own vector-pinned BLAKE3. Because keys have no accessor, the KAT lives where it can see bytes: an in-crate unit test in `crypto/src/session.rs`, or an observable form (the MAC of a fixed message and the keystream of a fixed buffer under the derived keys, both computable in the script given the derived bytes... which again needs BLAKE3 and ChaCha20 outside the crate). Prefer the in-crate unit test for the raw derived bytes.

**Seam trait** (lives in `port/src/chan/transport.rs`, which has `alloc`):

```rust
pub enum ExchangeStep { Continue, Done(crypto::key::Key) }

pub trait SessionExchange {
    /// Consume the peer's opaque exchange bytes, append this end's reply to `out`.
    fn step(&mut self, incoming: &[u8], out: &mut alloc::vec::Vec<u8>) -> crate::Result<ExchangeStep>;
}
```

Tests implement it with a struct that returns `Done(Key::from_bytes(fixed))` at once, and a second fixed value for the re-key.

### Pattern 6: Message bodies (discretion proposal) `[ASSUMED design]`

- **Base messages keep their 9P2000 bodies byte for byte** (`fcall.h` / `convS2M`), including their `fid[4]` fields. The `fid[4]` in a body is the client-scoped `fid_key` of `StatefulHandle`. This keeps WIRE-06's round trips checkable against a known format and invents as little wire as possible.
- **The header `FidHandle` carries the server-issued handle of the message's primary fid.** T-messages on an established fid (`Twalk`, `Topen`, `Tcreate`, `Tread`, `Twrite`, `Tclunk`, `Tremove`, `Tstat`, `Twstat`, `Tstream`) carry that fid's handle. R-messages that create a fid (`Rattach`, `Rwalk`, and `Rauth`) carry the new handle; that is how "`Twalk` returns `FidHandle`" works without changing `Rwalk`'s body. Messages with no fid (`Tattach` before any handle exists, `Tflush`, `Rerror`, the re-key pair) carry zero with `EXT_STATELESS_CAP` clear. A capability issued or presented travels in the `EXT_STATELESS_CAP` TLV with the header handle zero (D-12).
- **The four new pairs use the RFC section 6.2 formats literally** (`.planning/rfc-9p2000-aead.md:317-368`), minus the leading `size[4] type tag[2]`, which the envelope header already carries. Their semantics are Phase 6.
- **Data messages (D-09):** body = parameter block, then data padded with zeros to a multiple of 4096. The parameter block is the message's classic fixed fields (`Twrite`: `fid[4] offset[8] count[4]`; `Rread`: `count[4]`; `Rstream`: `stream_id[4] chunk_seq[8] count[4]`), with `count` the true length. The receiver checks `data_len % 4096 == 0` and `count <= data_len < count + 4096`.
- **Keystream placement:** parameters are encrypted from block counter 0; data page `i` is encrypted from block counter `1 + 64 * i` (a 4096-byte page is exactly 64 ChaCha20 blocks). Each page then decrypts on its own from `(key, nonce, counter)`, which is what Phase 5 needs to decrypt one page at fault time without the rest of the packet. Parameter blocks must fit one block (64 bytes); the three above are 16, 4 and 16. Non-data messages encrypt their whole body from counter 0. `chacha20_xor` already takes an explicit counter.

### Pattern 7: `fid.rs` (discretion proposal) `[ASSUMED design]`

Layouts (WIRE-07), from the RFC `[VERIFIED: .planning/rfc-9p2000-aead.md:213-232, 261-279]`:

- `StatefulHandle` wire form, 8 bytes: `fid_key` u32 LE, `ptr` u8 (non-zero), `shard_id` u8, `epoch` u16 LE.
- `StatelessCap` wire form, 24 bytes: `qid_path` u64 LE, `qid_vers` u32 LE, `qid_type` u8, `mode` u8, `epoch` u16 LE, `mac_tag` 8 bytes.
- `FidEntry`, 64 bytes, `#[repr(C, align(64))]`: `fid_key` u32, `client_id` u32, `qid_path` u64, `offset` u64, `qid_vers` u32, `iounit` u32, `lease_gen` u32, `ring_slot` u32, `dev_handler` u16, `epoch` u16, `qid_type` u8, `open_mode` u8, `flags` u16, `_reserved` [u8; 16]. Add `const _: () = assert!(size_of::<FidEntry>() == 64 && align_of::<FidEntry>() == 64);`.

`FidEntry` ↔ `[u64; 8]` by field-wise `to_le_bytes` / `from_le_bytes` packing. Compendium listing 9.6 uses `unsafe impl PodSlot` with `transmute` and a three-parameter `FlattenedTphtShard<u32, FidEntry, 8>`; neither exists in `ds` and neither is needed. The shard type is `FlattenedTphtShard<8, P>`.

```rust
pub struct FidShard<P: Probe = NoProbe> {
    tpht: FlattenedTphtShard<8, P>,          // home-only: with_banks_and_probe(blocks, 0, probe)
    active: Lock<RoaringBitmap>,             // slot ids
    reclaimed: Lock<RoaringBitmap>,
}
pub struct FidTable<P: Probe = NoProbe> {
    shards: Box<[FidShard<P>]>,              // at most 256 (shard_id is u8)
    domain_secret: crypto::key::Key,         // D-13
    epoch: u16,
}
```

- **Key:** `((client_id as u64) << 32) | fid_key as u64`. The server knows `client_id` from the connection, so the handle's `fid_key` is enough, and two clients' identical fid numbers do not collide.
- **Home-only shards.** Every entry must have a `TinyPtr` (F6), so the shards are built with zero banks. On `HomeFull` the table tries the next shard; `shard_id` in the handle records where the entry landed. All shards full is an error reply.
- **Resolve:** `shards[h.shard_id].tpht.deref_tiny(key, h.ptr)`, then compare in registers: `entry.fid_key == h.fid_key`, `entry.client_id == client_id`, `entry.epoch == h.epoch == table.epoch`, and a `LIVE` bit in `flags`. An out-of-range `shard_id` is rejected before the read. These comparisons read no further memory.
- **Single-read proof (WIRE-08):** a test `Probe` with two counters; assert `(arena_reads, header_reads) == (1, 0)` across one `resolve`, the exact pair Phase 3's DS-03 test asserts.
- **Bitmap ids (WIRE-09):** the arena slot id `(block << 4) | (ptr - 1)`, unique within a shard and dense. `port` cannot compute `block` without duplicating `ds` layout, so add two small safe methods to `FlattenedTphtShard`: `slot_id(&self, key, ptr) -> u32` and a bounds-checked `read_slot(&self, slot_id: u32) -> Option<[u64; WORDS]>`. The sweep is `let orphans = active.difference(&reclaimed);`, then for each id: `read_slot`, rebuild the key from the entry, `remove`. The test asserts the returned set equals a `BTreeSet` model of `active \ reclaimed` exactly, and that the survivors still resolve.
- **`StatelessCap`:** `mac_tag` = first 8 bytes of keyed BLAKE3 under the domain secret over a fixed 16-byte label (for example `b"r9 StatelessCap1"`) followed by the capability's first 16 bytes. Verification is one keyed hash and a constant-time compare; the table is not touched, which is the "zero FID memory" property. Test: two `FidTable` values built with the same `Key` verify each other's capabilities; a table with a different key rejects them; flipping any of the 24 bytes rejects; and the active bitmap and shard are unchanged across issue and verify.
- **`epoch` in this phase:** the table's current epoch (a constructor parameter). A capability or handle whose epoch differs is rejected. Bumping the epoch revokes everything issued before, which is the failover behaviour `Rversion`'s epoch will drive in Phase 6.
- **The capability is a bearer token** within the domain: it is not bound to a client or connection, by design (any replica must verify it with no session memory). With an 8-byte tag a forgery succeeds with probability 2^-64 per attempt, and an attempt needs an authenticated connection. Treat a failed capability as connection-fatal so guesses are not free.
- **`#G` path formula (WIRE-10):** a pure function `gpu_qid_path(ctx_id: u32, node_type: u8) -> u64` returning `((ctx_id & 0xFFFFFF) << 8) | (node_type & 0xFF)`, pinned with boundary values.

### Pattern 8: The `devmnt` branch (D-23, WIRE-12)

- Keep `Mnt::negotiate(io_r, io_w, offer)` with its exact signature and behaviour; it delegates to a new function with the version fixed to `VERSION9P`. The one kernel caller and the 14 test call sites then compile untouched, and the existing `devmnt_test.rs` suite is the "plain path unchanged" pin.
- Add the general form, for example `Mnt::negotiate_dialect(io_r, io_w, offer, want: Dialect) -> Result<Negotiated>` with `enum Negotiated { Plain(Mnt), Aead(chan::Handoff) }`. On an agreed `"9P2000.aead"` it hands the two io halves, the negotiated msize and the transcript bytes so far to the engine. It records the exact `Tversion` and `Rversion` bytes for the transcript.
- **Fail closed on downgrade.** `Rversion` travels in the clear, so anyone on the path can answer `"9P2000"`. When the caller asked for `"9P2000.aead"`, a plain answer is an error, not a silent fallback. (The transcript binding of D-17 only protects two ends that both reach the sealed phase.) The legacy fallback is Phase 6's gateway.
- The kernel never asks for aead this phase, so the branch is unreachable at boot. It is still linked, which is the byte change of F1.
- New test file `aead_negotiate.rs`: a fake answering `"9P2000.aead"` to an aead offer reaches the engine; the same fake answering `"9P2000"` to an aead offer is an error; a plain offer to either fake still yields a working `Mnt` (walk and read the canned tree).

### `cargo xtask test-9p` layout (WIRE-13)

Model it on `xtask/src/miri.rs`: a small module with a pure `command_specs()` that unit tests can inspect, and a refusal when the test files are missing (so a renamed file cannot turn the step into a silent pass).

Legs:

1. `cargo test --package crypto --lib session` and `cargo test --package crypto --test session`
2. `cargo test --package port --test aead_wire --test aead_tlv --test aead_transport --test aead_fid --test aead_negotiate`
3. Legs 1 and 2 again with the two soft-backend cfgs, passed in the leg's own `--config build.rustflags=[...]` exactly as the Phase 3 crypto leg does `[VERIFIED: xtask/src/main.rs:3629-3641]`. The host build links hardware SHA and NEON or AVX2 ChaCha20; the kernel runs the scalar code. The wire pins should run on both.

`cargo xtask test` already runs `cargo test --package port --lib --tests` and `--package crypto --package ds --lib --tests` `[VERIFIED: xtask/src/main.rs:3582-3604]`, so the new files run there too with no change, and hosted CI (`.github/workflows/xtask.yml` runs `cargo xtask test`) covers them. `test-9p` is the named subset the guide's verification matrix asks for.

### `forbid(unsafe_code)` and `UNSAFE-AUDIT.md`

- `port/src/lib.rs` gains `#[forbid(unsafe_code)] pub mod chan;` and `#[forbid(unsafe_code)] pub mod fid;`. Nothing in the proposal needs `unsafe`: `FlattenedTphtShard` is all atomics, `Lock<T>` is already `Sync`, the codec is `from_le_bytes`, and `FidEntry` is packed field by field. No `unsafe impl Send/Sync` is needed. **No new unsafe-holding module, so no IG-D07 approval.**
- `UNSAFE-AUDIT.md`: the row "every other port module | unsafe-free, `#[forbid(unsafe_code)]`" already covers the new modules `[VERIFIED: UNSAFE-AUDIT.md:197]`. Add a dated Phase 4 delta paragraph: `port` lint-visible sites stay 23; `crypto` stays 0; `ds` stays 1 (or the new line number of the `deref_tiny` block if `flattened.rs` shifts — it is recorded as `flattened.rs:440` today); the scope sentence gains "`port` now links `crypto/` and `ds/`, so the pinned upstream crates' unsafe is reachable from the kernel image"; and the third-party record is unchanged.
- If `ds/src/tpht/flattened.rs` is edited (slot clearing, `slot_id`, `read_slot`), the audit's line reference must be re-measured and `cargo xtask miri` re-run. Phase 3's Miri run takes 230 to 402 s, an accepted deviation.
- `xtask/src/lintcontract.rs` gains nothing for `port`'s forbids; the compiler enforces them.

### Documents this phase edits (D-10)

| Document | Edit |
|----------|------|
| `.planning/rfc-9p2000-aead.md` | Dated notes at 3.2 (per-connection per-direction keys, teardown on connection close, derivation formula), 4.1/4.2 (header length per C1, `Pad` must be 0), 4.3 (MAC covers every byte before the MAC), 4.4 (sixth bit, TLV record format), 5 (capability travels in a TLV), 6.1 (106 to 109, and 138/139 if adopted), 6.2 (data-part rule, C4), 11 (re-key cutover) |
| `.planning/PROJECT.md` | Dated Key Decisions rows naming the superseded wording; W1, W2, W4 leave Open Questions; N1 fully settled; N5 and N11 annotated; IG-D11 row per the owner's checkpoint answer |
| `.planning/STATE.md` | W1, W2, W4 leave the open-question table |
| `.planning/REQUIREMENTS.md` | WIRE-02 amended to six bits |
| `.planning/r9-architecture-and-code-v4.md` | Listing 9.9 rewritten in place with a dated note |
| `docs/threat-model.md` | Optional: rows at lines 136 and 142 say "not implemented"; W3/W4 notes at 171-172. It is a living document outside the lock set. |

### The x86_64 byte-change procedure, as run in Phase 2

From `02-OWNER-STOP.md` and Phase 2 D-23 to D-29. This is planned work here (D-25).

1. **Order.** Phase-end code review first (D-24), then fixes, then this stop. Any later commit that changes kernel bytes voids the hash commit (the "one-commit rule"), so the stop comes after the last code change.
2. **Mac evidence before the stop.** Before/after record on one venue (`before.sha256` format: `base`, `venue`, one line per config), per F1's evidence set. The baseline at `6263543` on this Mac is `virtshell 9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e`, `q35churnshell c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b` `[MEASURED]`; re-take it at the phase's actual base commit.
3. **aarch64 quick pass on the Mac:** `nix run .#boot-r9-tier1` (HVF), `R9_ACCEL=tcg nix run .#boot-r9-tier1`, `nix run .#boot-r9-smoke`, `nix run .#boot-r9-net-smoke`, `nix run .#boot-r9-cpu`; each passes on exit 0 with `ALL STAGES PASS`. Then the Mac suite: `cargo xtask test`, `cargo test -p xtask`, `cargo xtask check`, `cargo xtask clippy` (default arch; the `--arch aarch64` clippy form fails at HEAD on 7 pre-existing sites and is not a criterion).
4. **Show the unpushed commits; the owner approves pushing a branch** (for example `phase-04/aead-wire`), not `main` (D-26).
5. **On the owner's x86_64 Linux host,** with every verdict command wrapped in the `run_logged` function so each log starts `HEAD=` and ends `EXIT=`:
   a. fetch and switch to the branch; confirm `git rev-parse HEAD` equals the pushed tip.
   b. The x86_64 host tests first (D-29): `nix develop -c cargo xtask test`, plus `cargo xtask test-9p`. New code compiles there for the first time; running it before the hash keeps a compile fix from voiding a hash commit.
   c. `nix build .#kernel-x64` and `nix build .#kernel-virt` (the sandboxed derivations vendor `Cargo.lock` through `importCargoLock`; the lock changes by two lines and adds no crate, so this is low risk, but it is the first sandboxed build with `port` linking `crypto`).
   d. `nix build .#boot-iso`: expected to fail with `hash mismatch in fixed-output derivation` and a `got:` value. The owner replaces the value on `flake.nix` line 983, which reads today `outputHash = "sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=";` `[VERIFIED: flake.nix:983 via grep -n this session]`, adds a dated comment, and commits `flake.nix` alone, with a message the plan supplies and no attribution line (D-28). Then the plain build and `--rebuild` (the witness driver's `kernel` step prints `KERNEL-BOOTISO-BUILD-OK` and `KERNEL-BOOTISO-REBUILD-OK`). Push the hash commit to the branch.
   e. `nix develop -c cargo xtask gate --arch x86-64 --accel kvm` once (boot to rio).
   f. `nix develop -c cargo xtask gate --arch x86-64 --accel tcg` (the byte-change rule's TCG gate; see C6 on the venue).
   g. The full gate set: `cargo xtask gate --arch x86-64 --full --accel kvm`.
   h. Report back the logs.
6. **`main` is pushed only after the gates are green and the owner approves again.**

### Anti-Patterns to Avoid

- **Copying compendium listings 9.6 or 9.9.** 9.9's `AeadHeader` is `repr(C, packed)`, holds a Rust enum as `fid_handle`, has no `Pad` or `ExtTLVs Length`, and its `MsgType` omits eight opcodes. 9.6 uses `transmute`, `spin` and a `roaring` crate that is a dev-dependency of `ds` only.
- **Exposing key bytes to `port`.** No `Key::expose`, no `DirectionKeys::mac_key()`.
- **A global soft-backend rustflag in `.cargo/config.toml`.** Lint rule R5 forbids it and it would reach the kernel builds.
- **Reading `ExtTLVs Length`, `Type` or `SeqNonce` before the MAC is verified.**
- **Silent fallback to plain 9P2000 when aead was asked for.**
- **Wrapping the send counter, or reusing a generation's keys after the counter returns to 0.**
- **Per-plan owner diff stops for `port/src`.** D-24 waives them for this phase.
- **Locking IG-D11 in PROJECT.md without the owner's answer.**

## Don't Hand-Roll

| Problem | Don't build | Use instead | Why |
|---------|-------------|-------------|-----|
| HKDF, HMAC | A second derivation | `crypto::kdf::{hkdf_extract, hkdf_expand}` | RFC 5869 and 4231 pinned on both backends |
| MAC | HMAC over BLAKE3, Poly1305 | `crypto::mac` keyed mode | D-01 |
| Stream cipher | A ChaCha20 in `port` (one exists in `devcons`) | `crypto::page::chacha20_xor` | RFC 8439 pinned; explicit counter |
| Constant-time compare | `==` on tag slices | the accumulator fold in `crypto` | No early exit |
| Key storage and scrubbing | Manual zero loops | `crypto::key::Key` (`Zeroizing`) | Zeroizes on drop |
| Hash table | A `BTreeMap` or `HashMap` fid table | `ds::tpht::flattened::FlattenedTphtShard` | D-05, WIRE-08 |
| Set difference | A loop over fids | `RoaringBitmap::difference` / `andnot` | WIRE-09 says one and-not |
| Locks | `spin` | `port::mcslock::Lock` | Already the `port` primitive |
| Plain 9P framing | A second size-prefixed reader | the `frame_one` discipline (size bounds before body) | Already hardened against oversized claims |

**Key insight:** every primitive is already pinned. The work of this phase is composition, and the place composition goes wrong is at the edges: nonce reuse, verify-after-use, stale handles, and silent downgrade.

## Common Pitfalls

### Pitfall 1: The header is 28 bytes, not 24
**What goes wrong:** A pin written to "24" either drops a field or mis-places `ExtMask`. **Avoid:** resolve C1 with the owner before the first wire pin; pin every field offset (0, 4, 5, 7, 8, 16, 24, 26) as well as the total.

### Pitfall 2: A clunked handle still resolves
**What goes wrong:** `remove` clears the header slot but not the arena words, and `deref_tiny` never looks at the header, so `resolve` returns the old `FidEntry`, which still matches `fid_key`, `client_id` and `epoch`. **Why:** documented behaviour of `deref_tiny` ("A stale pair returns whatever the slot now holds"). **Avoid:** have `ds` zero the slot's words on `remove` (under the block's seqlock, in safe code), and require a `LIVE` bit in `FidEntry.flags` so an all-zero slot never validates. Re-run Miri. **Warning sign:** a test that clunks a fid and then resolves the old handle gets `Some`.

### Pitfall 3: A spilled entry has no handle
**What goes wrong:** with the default constructor a full home block spills to the banks and `insert` returns `Ok(None)`; there is no `TinyPtr` to put in a `StatefulHandle`. **Avoid:** home-only shards (`with_banks(n, 0)`) and fall over to another shard on `HomeFull`.

### Pitfall 4: Torn read on the single-read path
**What goes wrong:** `deref_tiny` takes no seqlock and loads eight words `Relaxed`, so a read that races an insert into the same slot can mix two entries. **Why it is tolerable:** the server hands out a handle only after `insert` returns, so the race needs a client to use a stale handle while reusing the same fid number; both entries are that client's own, and the key includes `client_id`. **Avoid making it worse:** keep `fid_key` and `client_id` in word 0 (they are, in the RFC layout) so the identity check is one atomic word. Document the property; do not claim a consistent snapshot.

### Pitfall 5: Keystream reuse across a re-key
**What goes wrong:** the counter returns to 0 after a cutover; if the new keys equal the old (a seam returning the same secret), every nonce repeats under the same key. **Avoid:** `generation` in the HKDF `info` (Pattern 5), and a test that feeds the same secret twice and asserts the two generations' keys differ.

### Pitfall 6: "Zeroized" has limits
**What goes wrong:** the long-lived state (`Key`s, prefix, counters) is scrubbed, but each per-packet `blake3::Hasher` and `ChaCha20` cipher holds a copy of the key on the stack and is dropped without scrubbing. `blake3`'s `zeroize` feature is off (`crypto/Cargo.toml` enables only `pure` and the five `no_*` features; the feature exists as `zeroize = ["dep:zeroize", "arrayvec/zeroize"]` `[VERIFIED: blake3-1.8.7/Cargo.toml features in the vendored registry source]`). **Avoid overclaiming:** state in `session.rs` and in the WIRE-05 test names what is zeroized. **Optional hardening, owner's choice:** enable the feature and scrub the hasher in `PacketMac`'s drop. It changes a pinned crate's feature set, so it follows Phase 3's provenance-comment rule.
**How to test zeroization:** key bytes are unreadable outside `crypto`, so the byte-level assertion is an in-crate unit test in `crypto/src/session.rs` (teardown, then every key byte and the prefix are zero). The `port` test asserts the observable: after teardown or a MAC failure, `seal` and `open` return `Err(Closed)`.

### Pitfall 7: `cargo xtask test` goes red the moment the edge is added
**What goes wrong:** rule R4. **Avoid:** change R4 and its fixtures in the same commit as the two `port/Cargo.toml` lines. A plan that adds the edge without the rule change cannot reach GREEN.

### Pitfall 8: The first `port/src` commit changes kernel bytes for the rest of the phase
**What goes wrong:** from that commit on, `main` holds a kernel whose `boot-iso` `outputHash` in `flake.nix` is stale on x86_64 Linux, until the owner stop. **Why it is acceptable:** D-25 accepts the byte change and nothing is pushed before the stop; Phase 2 ran the same way. **Avoid surprises:** put the dependency edge and the `devmnt` branch late, so most plans (`crypto::session`, and `chan`/`fid` as pure library code) change nothing a booted kernel links. Note that `pub mod chan;` alone links nothing unless something reachable calls into it.

### Pitfall 9: The 2 MiB `.text` unit on x86_64
**What goes wrong:** x86_64 `.text` is aligned to 2 MiB; text symbols total about 1.03 MB after the primitives. Past 2 MiB the section doubles and every later address moves. **Avoid:** record the text symbol total in the byte evidence and treat crossing 2,097,152 as a stop to look at, not a silent pass.

### Pitfall 10: Transcript order
**What goes wrong:** the two ends hash the handshake messages in different orders and derive different keys, and the first sealed packet fails with no useful error. **Avoid:** strictly serial handshake; hash whole messages in wire order; one test where the server alters msize in `Rversion` after the client sent it (keys differ, first sealed packet fails, connection closed) and one where nothing is altered.

### Pitfall 11: RED pins must fail for the right reason
**What goes wrong:** a RED test that fails to compile because the module does not exist proves nothing about the behaviour (IG-V01, D-03). **Avoid:** as in Phase 3, land the type and function signatures with `todo!()`-free stub bodies that return a wrong value or an error, commit the failing test, then implement. Record RED and both GREEN runs in the phase evidence.

## Code Examples

### Sealing with the proposed `crypto::session` API

```rust
// Sketch [ASSUMED design].  Builds only on calls that exist today:
// Blake3::new_keyed / update / finalize (crypto/src/mac.rs:59-71) and
// chacha20_xor (crypto/src/page.rs).

pub struct DirectionKeys { k_enc: Key, k_mac: Key, prefix: [u8; 4] }   // private fields

pub struct PacketMac { h: crate::mac::Blake3 }

impl DirectionKeys {
    pub fn mac(&self) -> PacketMac {
        PacketMac { h: crate::mac::Blake3::new_keyed(self.k_mac.bytes()) }
    }
    /// XOR the keystream for sequence `seq` into `buf`, starting at block `counter`.
    pub fn keystream(&self, seq: u64, counter: u32, buf: &mut [u8]) -> crate::Result<()> {
        let mut nonce = [0u8; 12];
        nonce[..4].copy_from_slice(&self.prefix);
        nonce[4..].copy_from_slice(&seq.to_le_bytes());
        crate::page::chacha20_xor(self.k_enc.bytes(), &nonce, counter, buf)
    }
}

impl PacketMac {
    pub fn update(&mut self, part: &[u8]) { self.h.update(part); }
    pub fn finalize(self) -> [u8; 32] { self.h.finalize() }
    pub fn verify(self, tag: &[u8; 32]) -> crate::Result<()> {
        let got = self.h.finalize();
        let mut acc = 0u8;
        for (a, b) in got.iter().zip(tag) { acc |= a ^ b; }
        if acc == 0 { Ok(()) } else { Err(crate::Error::TagMismatch) }
    }
}
```

### The splice and replay pins (WIRE-11), in outline

```rust
// Two connections with different session secrets over two in-memory links.
// 1. Capture a sealed Twrite from connection A.
// 2. Replay: deliver the captured bytes to A's server a second time.
//    Expect: not delivered to the handler; connection A closed; keys zero.
// 3. Splice within a connection: on a fresh A, flip one byte of the cleartext
//    FidHandle field (packet offset 8..16) and deliver.
//    Expect: MAC failure; the handler's decode counter is still 0.
// 4. Splice across connections: deliver A's packet to B's server.
//    Expect: MAC failure (different k_mac).
// 5. Reflect: deliver a client-to-server packet back to the client.
//    Expect: MAC failure (per-direction keys).
// 6. For every byte offset in 0..size-32 of one packet: flip it, deliver to a
//    fresh connection, expect MAC failure.  (Success criterion 2: "any
//    authenticated byte".)
```

## State of the Art

| Old approach | Current approach | When | Impact here |
|--------------|------------------|------|-------------|
| AAD = seven named fields (RFC 4.3) | MAC over every byte before the MAC | D-07, 2026-11-24 | No field can be left out by a formula |
| Per-FID transient key, dropped on `Tclunk` (RFC 3.2) | Per-connection, per-direction keys, dropped on teardown | D-15, 2026-11-24 | One counter per direction; the FID is bound by the MAC |
| 24-byte capability "via token indirection" | A TLV under a sixth `ExtMask` bit | D-11, 2026-11-24 | Header handle stays 8 bytes |
| Explicit per-record nonce on the wire (TLS 1.2 AEAD) | Derived per-direction IV, sequence number implicit or carried (TLS 1.3, WireGuard) | 2018 | D-08 follows it `[CITED: rfc8446 section 5.3; wireguard.com/protocol]` |

**Deprecated in the sources:** "BLAKE3 / Poly1305" (guide 6.3), "64-bit block counters" (RFC 3.1, amended in Phase 3), compendium listing 9.9's `AeadHeader`.

## Assumptions Log

| # | Claim | Section | Risk if wrong |
|---|-------|---------|---------------|
| A1 | The header is 28 bytes (the field list governs, "24" is a slip) | C1, Pattern 1 | One-way wire contract; every offset after `Tag` |
| A2 | `EXT_STATELESS_CAP = 1 << 5`; TLV record = `type[1] rsvd[1] len[2] value pad-to-8`; ascending order; unknown bits fatal | Pattern 3 | One-way wire contract |
| A3 | Session `info` layout, direction and purpose bytes, `generation` in `info`, BLAKE3 transcript hash chained across re-keys | Pattern 5 | One-way (D-17); a different layout gives different keys |
| A4 | Re-key messages are opcodes 138/139 with `kind[1] generation[4] confirm[32]`, tag `NOTAG`; `confirm` is a keyed hash under the new MAC key | Pattern 4 | Extends the opcode registry; one-way |
| A5 | `REKEY_AFTER_PACKETS = 1 << 48`, `REKEY_ACK_TIMEOUT_MS = 5000`, never send sequence `u64::MAX` | Pattern 4 | Tunable constants; low risk |
| A6 | An unexpected sequence number closes the connection (not drop-only) | Pattern 2 | Behavioural; WIRE-11 says "dropped" |
| A7 | Base message bodies are unchanged 9P2000 bodies; the header handle carries the primary fid's server handle; R-messages that create a fid return the handle in the header | Pattern 6 | Shapes Phase 6's client and server; costly |
| A8 | Data pages are encrypted from block counter `1 + 64 * i`, parameters from 0 | Pattern 6 | Costly: Phase 5 DMA ingress and fault-time decrypt build on it |
| A9 | `Tauth`/`Rauth` keep standard bodies; opaque exchange bytes ride `afid` reads and writes | C5 | Changes what the transcript contains |
| A10 | TPHT key is `(client_id << 32) | fid_key`; shards are home-only; bitmap ids are slot ids; `ds` gains `slot_id`, `read_slot` and slot clearing on `remove` | Pattern 7, Pitfall 2 | Touches Phase 3 code in `ds/src/tpht/`; Miri must be re-run |
| A11 | Capability tag covers a fixed label plus the capability's first 16 bytes; `epoch` is the table's current epoch; a failed capability closes the connection | Pattern 7 | Bearer-token property; Phase 6 may want a user binding |
| A12 | Volatility classes are not written into `dat::Qid.typ` in this phase | C2 | WIRE-10's wording says "as `qtype` bits"; the owner may read it differently |
| A13 | The codec covers 106 to 109 as well as the RFC registry | C3 | Low |
| A14 | A plain `Rversion` to an aead offer is an error | Pattern 8 | Low; Phase 6 adds the gateway |
| A15 | The cause of the `.text` change with the edge alone is `port`'s crate hash changing mangled names and link order | F1 | None for the plan; the measurement stands either way |
| A16 | smoltcp's `poll` takes a caller-supplied timestamp, as used in `port/src/devip` | F4 | None; the recommendation does not depend on it |
| A17 | The engine's own code adds well under the 1 MB of x86_64 `.text` headroom | Pitfall 9 | If wrong, `.text` grows to 4 MiB; planned as a measured stop |

## Open Questions

1. **C1 to C5 above.** Owner checkpoint before the first wire pin.
2. **IG-D11 lock.** Not decided (CONTEXT). The plan must ask; a natural place is the same checkpoint, or the D-10 documents plan. Recommendation to offer: lock amended, with the amendments named (MAC scope, key scope, capability TLV, nonce rule), and the still-open items listed as outside the lock (real key exchange N5, page-level check N11, qtype bits C2).
3. **Does the owner want `blake3`'s `zeroize` feature on?** (Pitfall 6.) What we know: without it, per-packet hasher state is not scrubbed. Recommendation: ask at the checkpoint; default to off and state the limit.
4. **Where does `client_id` come from?** `FidEntry.client_id` is "Authenticated client node ID". This phase has no authentication, so the test server assigns one per connection. Phase 6 defines the real source.
5. **Who drives `poll(now)` in a kernel?** Nobody this phase (D-23). Recorded so Phase 6 does not assume the deadline fires by itself.
6. **Should `test-9p` also be a CI step?** `cargo xtask test` already runs the same files. Phase 2's review item WR-01 (xtask's own unit tests are not in `cargo xtask test` or CI) is still open and applies to the new `test9p.rs` and `lintcontract.rs` tests: run `cargo test -p xtask` in the validation set.

## Environment Availability

| Dependency | Required by | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Nix devshell | every build and test | ✓ | Determinate Nix 3.24.3 (nix 2.35.6) | — |
| Rust toolchain (devshell) | all | ✓ | rustc 1.95.0-nightly (2026-01-29) | — |
| Miri toolchain (`R9_MIRI_TOOLCHAIN`) | `cargo xtask miri` if `ds/src/tpht/` is edited | ✓ | exported by the devshell | — |
| Python 3 | the session-derivation KAT script | ✓ | 3.13.18 (host and devshell) | Node is also present |
| QEMU with HVF | aarch64 quick pass | ✓ | 11.1.2 in the devshell | — |
| x86_64 Linux host with KVM | byte-change stop (hash commit, gates) | ✗ on this machine | — | None: it is the owner's host, reached by a pushed branch |
| Docker | only if the owner wants the TCG gate in a container (C6) | ✓ | `/usr/local/bin/docker` present | TCG gate on the Linux host |
| lima | not needed this phase | ✗ | — | — |

**Missing dependencies with no fallback:** the owner's x86_64 Linux host. It is an owner stop by design (D-25), not something a plan can automate.

## Validation Architecture

`workflow.nyquist_validation` is absent from `.planning/config.json`, so validation is enabled.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in tests through `cargo xtask`; Miri for `ds/src/tpht/` when edited |
| Config file | none; workspace `Cargo.toml` and `.planning/config.json` (`build_command`, `test_command`) |
| Quick run command | `nix develop -c cargo xtask test-9p` (once it exists); until then `nix develop -c cargo test -p crypto --lib session` or `nix develop -c cargo test -p port --test <file>` |
| Full suite command | `nix develop -c cargo xtask test && nix develop -c cargo test -p xtask && nix develop -c cargo xtask check && nix develop -c cargo xtask clippy` (plus `nix develop -c cargo xtask miri` when `ds/src/tpht/` changed) |
| Estimated runtime | quick: seconds once built; full: a few minutes; Miri 230 to 402 s (Phase 3 measurement, accepted deviation) |

### Phase Requirements → Test Map

File names are the proposal of "Recommended Project Structure"; test function names are for the planner to fix.

| Req ID | Behavior | Test type | Automated command | File exists? |
|--------|----------|-----------|-------------------|--------------|
| WIRE-01 | Header fields at pinned offsets and total length; encode then decode is identity; a hand-written byte vector decodes to the expected fields | unit | `cargo test -p port --test aead_wire header` | ❌ Wave 0 |
| WIRE-01 | Listing 9.9 rewritten with a dated note; RFC notes present | doc check | `grep -c "Amended 2026-" .planning/rfc-9p2000-aead.md` and a grep for the new header in the compendium | ❌ |
| WIRE-02 | Each of six extensions round-trips; all 64 mask combinations parse; `ExtTLVs Length == 0` iff mask 0; unknown bit, wrong order, duplicate, bad `len`, non-zero pad each rejected; parser allocates nothing | unit | `cargo test -p port --test aead_tlv` | ❌ Wave 0 |
| WIRE-03 | Flip each byte of `[0, size-32)` and of the MAC: connection closed, decode counter 0; later `seal`/`open` return closed | integration | `cargo test -p port --test aead_transport mac_failure` | ❌ Wave 0 |
| WIRE-03 | After failure every key byte and the prefix are zero | unit (in-crate) | `cargo test -p crypto --lib session` | ❌ Wave 0 |
| WIRE-04 | Counter starts at 0 and increments by 1; `n-1`, `n+1` and a repeat of `n` are rejected; re-key fires at a low threshold; marker, ack with `confirm`, no old-key packet accepted after the marker; ack timeout closes; counter never wraps | integration | `cargo test -p port --test aead_transport rekey` and `... sequence` | ❌ Wave 0 |
| WIRE-05 | Sealed `Tattach` before `Tauth` is refused; derivation KAT; the two directions' keys differ; altered `Rversion` gives a failing first packet; teardown zeroizes | unit + integration | `cargo test -p crypto --test session` and `cargo test -p port --test aead_transport handshake` | ❌ Wave 0 |
| WIRE-06 | Every opcode 100 to 127 (106 illegal) and 130 to 137 encodes and decodes to an equal value; registry table pinned; data bodies are whole pages with the true count; short, exact and multi-page data | unit | `cargo test -p port --test aead_wire messages` | ❌ Wave 0 |
| WIRE-07 | `size_of` 8, 24, 64; `align_of::<FidEntry>() == 64`; wire forms round-trip; a `StatefulHandle` is never all zero; cap bit set with a non-zero header handle is rejected | unit | `cargo test -p port --test aead_fid layout` | ❌ Wave 0 |
| WIRE-08 | Counting probe shows `(1, 0)` for a resolve; wrong client, wrong epoch and clunked handle do not resolve; two tables with one key verify each other's capabilities with the shard and bitmaps untouched; a different key rejects | unit | `cargo test -p port --test aead_fid resolve` and `... stateless` | ❌ Wave 0 |
| WIRE-09 | Sweep result equals a `BTreeSet` model of `active \ reclaimed` on randomized inputs; reclaimed fids still resolve, orphans do not | unit | `cargo test -p port --test aead_fid sweep` | ❌ Wave 0 |
| WIRE-10 | 13-byte `Qid` round trip; the three class values; two class bits at once rejected; `#G` path formula at boundary values; the C2 collision pinned | unit | `cargo test -p port --test aead_fid qid` | ❌ Wave 0 |
| WIRE-11 | Replay dropped; edited `FidHandle` fails the MAC; cross-connection and reflected packets fail | integration | `cargo test -p port --test aead_transport splice` | ❌ Wave 0 |
| WIRE-12 | aead offer reaches the engine; plain offer yields a working `Mnt`; downgrade is an error; the existing suite is unchanged and green | integration | `cargo test -p port --test aead_negotiate` and `cargo test -p port --test devmnt_test` | ❌ Wave 0 (new file); ✅ `devmnt_test.rs` |
| WIRE-12 | The system still boots to rio | gate, owner host | `nix develop -c cargo xtask gate --arch x86-64 --accel kvm` | ✅ (manual: owner stop) |
| WIRE-13 | The subcommand exists, runs the listed files, and refuses when one is missing | unit + smoke | `cargo test -p xtask test9p` and `nix develop -c cargo xtask test-9p` | ❌ Wave 0 |
| (F2) | An arch crate naming `crypto` is still rejected; `port` naming it is accepted | unit | `cargo test -p xtask lintcontract` | ✅ (fixtures to change) |
| (D-25) | Kernel byte evidence recorded; FP/SIMD scan equal to baseline; x86_64 text under 2 MiB | evidence | grep of `VERDICT` lines in the phase's `evidence/kernel-bytes/KERNEL-BYTES.md` | ❌ |

### Sampling Rate
- **Per task commit:** the single test file the task touches, then `nix develop -c cargo xtask test`.
- **Per wave merge:** `cargo xtask test`, `cargo test -p xtask`, `cargo xtask check`, `cargo xtask clippy`; `cargo xtask miri` if `ds/src/tpht/` changed in the wave.
- **Phase gate:** full suite green twice (IG-V01); `cargo xtask test-9p` green twice on default and scalar backends; kernel byte evidence recorded; aarch64 quick pass; then the owner stop.
- **Feedback latency:** about 120 s once built, Miri excepted as in Phase 3.

### Wave 0 Gaps
- [ ] `crypto/src/session.rs` and `crypto/tests/session.rs` — WIRE-05, WIRE-03 (zeroize)
- [ ] `port/tests/common/aead_link.rs` — in-memory duplex link, client and server ends, fake clock, decode counter
- [ ] `port/tests/aead_wire.rs`, `aead_tlv.rs`, `aead_transport.rs`, `aead_fid.rs`, `aead_negotiate.rs`
- [ ] `xtask/src/test9p.rs` and its subcommand — WIRE-13
- [ ] `xtask/src/lintcontract.rs` rule R4 and fixtures — must land with the dependency edge
- [ ] Signature stubs for `chan`, `fid` and `session` so RED pins fail on behaviour, not on a missing module (Pitfall 11)
- [ ] `evidence/kernel-bytes/before.sha256` at the phase's base commit

### Manual-only verifications
- The `boot-iso` `outputHash` commit, the KVM and TCG gates and the full gate set: the owner's x86_64 Linux host.
- Branch push and `main` push: owner approval each time.
- Phase-end code review of the whole `port/src` change (D-24).
- The IG-D11 lock and C1 to C5: owner checkpoint.

## Security Domain

`security_enforcement` is absent from `.planning/config.json`, so this section applies. ASVS is a web-application standard; its categories are mapped to the nearest thing a wire protocol has.

### Applicable ASVS Categories

| ASVS category | Applies | Control in this phase |
|---------------|---------|-----------------------|
| V2 Authentication | partly | The session secret comes from a seam (D-16). No real peer authentication exists; this is a stated limit, not a control. |
| V3 Session Management | yes | Per-connection, per-direction keys; strict sequence numbers; re-key before the counter limit; zeroize on teardown and on any failure |
| V4 Access Control | partly | `StatelessCap` verification under the domain secret; `StatefulHandle` validated against `client_id` and `epoch`. Real permission checks are later phases. |
| V5 Input Validation | yes | `Size` bounded by msize before the body is read; every length field checked against the remaining bytes; `Pad`, reserved bytes and padding must be zero; unknown bits and opcodes rejected; no `unwrap` on wire data |
| V6 Cryptography | yes | Only `crypto/`'s pinned primitives; no new primitive; constant-time tag comparison; no key bytes outside `crypto/` |
| V9 Communications | yes | Encrypt-then-MAC over the whole packet; transcript-bound keys; fail closed on downgrade |

### Known Threat Patterns

| Pattern | STRIDE | Mitigation |
|---------|--------|------------|
| Splicing a payload onto another fid by editing the cleartext handle | Tampering | The MAC covers the header (D-07) |
| Splicing across connections or reflecting a packet to its sender | Tampering, Spoofing | Per-connection and per-direction keys (D-15) |
| Replay or reordering | Tampering | Strict next-in-sequence check after MAC verification |
| Acting on unauthenticated fields | Tampering | `Unverified` → `Verified` types; only `Size` is read early, for bounds |
| Version or msize tampering in the cleartext handshake | Tampering | Transcript hash in the derivation (D-17); plain answer to an aead offer is an error |
| Keystream reuse (nonce reuse) | Information disclosure | Counter never wraps; re-key threshold; `generation` in `info`; derived per-direction prefix |
| Keys left in memory after teardown | Information disclosure | `Zeroizing` keys; explicit `fail()`/teardown; limits stated (Pitfall 6) |
| Oversized `Size` or TLV length causing allocation or over-read | Denial of service | Bounds before allocation, as `frame_one` does |
| Forged capability | Elevation of privilege | 8-byte keyed tag under the domain secret; failure closes the connection; `epoch` check |
| Stale handle used after clunk or failover | Elevation of privilege | Slot cleared on remove plus `LIVE` flag; `epoch` compare; sweep frees orphans |
| One client presenting another client's handle | Elevation of privilege | `client_id` is part of the TPHT key and is compared after the read |
| Timing leak in tag comparison | Information disclosure | Constant-time accumulator fold |
| A peer that never acknowledges a re-key | Denial of service | Ack timeout closes the connection (D-20) |
| Compromised exchange seam or factotum choosing the secret | Spoofing | Out of scope this phase (D-16); listed in the threat model as untreated |

## Sources

### Primary (HIGH confidence)
- Repository at `6263543`, opened this session: `port/src/devmnt.rs`, `port/src/dat.rs`, `port/src/lib.rs`, `port/src/devcap.rs`, `port/tests/devmnt_test.rs`, `crypto/src/{lib,mac,key,kdf,page}.rs`, `crypto/Cargo.toml`, `ds/src/lib.rs`, `ds/src/tpht/{mod,flattened,feistel}.rs`, `ds/src/roaring.rs`, `xtask/src/main.rs` (TestStep, CheckStep, BuildStep), `xtask/src/lintcontract.rs`, `xtask/src/miri.rs`, `UNSAFE-AUDIT.md`, `x86_64/lib/kernel.ld`, `lib/x86_64-unknown-none-elf.json`, `flake.nix`
- `../harvey/sys/include/fcall.h`, `../harvey/sys/include/libc.h` (read-only reference)
- Planning documents: `.planning/rfc-9p2000-aead.md`, the guide and compendium at the lines CONTEXT names, `.planning/INGEST-CONFLICTS.md`, `.planning/PROJECT.md`, `.planning/intel/constraints.md`, `docs/threat-model.md`, Phase 2 `02-CONTEXT.md`, `02-OWNER-STOP.md`, `02-05-PLAN.md`, evidence; Phase 3 `03-CONTEXT.md`, `03-RESEARCH.md` (F2, F3), `03-OWNER-STOP.md`, `03-VALIDATION.md`, evidence
- Vendored registry sources: `blake3-1.8.7` (`src/lib.rs`, `src/platform.rs`, `build.rs`, `Cargo.toml`)
- Measurements made this session in a throwaway clone (six kernel builds, `cargo xtask check`, riscv64 build, `cargo xtask test`, symbol-size and disassembly scans)

### Secondary (MEDIUM confidence)
- RFC 8446, sections 4.4.1, 4.6.3, 5.3, 7.1, 7.3 — https://www.rfc-editor.org/rfc/rfc8446.txt (fetched; quoted passages returned by the fetch tool)
- WireGuard protocol page — https://www.wireguard.com/protocol/ (nonce composition; the page gave no numeric value for its re-key constants, so none is cited)

### Tertiary (LOW confidence)
- None used as a basis for a recommendation. Context7 was not available in this environment and `ctx7` is not installed; the one library question (BLAKE3's incremental API) was answered from the vendored source instead.

## Metadata

**Confidence breakdown:**
- Existing code and its gaps: HIGH — read directly, values quoted
- Kernel build and byte change: HIGH — measured on one venue; the Linux sandboxed build is not measured and stays an owner-stop item
- Discretion proposals (TLV, derivation, re-key encoding, bodies, `FidShard`): MEDIUM — internally consistent and built only on existing APIs, but they are proposals and several are one-way
- Byte-change procedure: HIGH — transcribed from the Phase 2 and Phase 3 owner-stop pages

**Research date:** 2026-11-24
**Valid until:** about 30 days for the code findings (until `port/`, `crypto/`, `ds/` or `xtask/` change); the contradictions C1 to C5 are valid until the owner answers them
