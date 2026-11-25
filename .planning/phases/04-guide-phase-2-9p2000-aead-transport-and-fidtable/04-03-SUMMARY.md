---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "03"
subsystem: kernel-transport-crypto
tags: [9p2000-aead, hkdf, blake3, transcript, zeroize, stateless-cap, rekey, known-answer-vector]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "crypto::kdf (hkdf_extract/expand), crypto::mac (Blake3 unkeyed/keyed, blake3_keyed, verify_keyed), crypto::key::Key, crypto::page::chacha20_xor; the Phase 3 KAT-in-a-ledger precedent"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-02)
    provides: "crypto::session::derive, DirectionKeys/SessionKeys with private fields, PacketMac, DirectionKeys::keystream; the sealed-packet tracer"
provides:
  - "crypto::session::Transcript — unkeyed BLAKE3 over whole handshake messages in wire order, chainable across re-keys"
  - "DirectionKeys/SessionKeys::zeroize and ::is_zeroized — byte-level teardown scrubbing with a non-leaking bool query"
  - "crypto::session::cap_tag/cap_verify — the 8-byte StatelessCap tag under a caller-supplied domain Key"
  - "DirectionKeys::rekey_confirm/verify_rekey_confirm — keyed BLAKE3 under the MAC key over r9 rekey ack || generation"
  - "the independent Python KAT for the six derivation outputs, plus six in-crate and eleven public pins"
affects: [04-07 transport state machine, 04-08 re-key, 04-09/04-10 fid table StatelessCap, 04-13 RFC formulas, 04-17 test-9p]

actuals:
  tokens: 8995
  tasks: 2
  commits: 6
  plan_head_before: f2455ef104611209b24b3a1c29006e93229b58e1
  plan_head_after: 9d702a9

tech-stack:
  added: []
  patterns:
    - "independent known-answer vector: Python hmac/hashlib computes the six derivation outputs; constants copied character for character into the in-crate test"
    - "in-crate test module for raw key bytes, public integration tests for behaviour: Key::bytes is pub(crate), so byte-level pins live in crypto/src/session.rs"
    - "is_zeroized folds all 68 covered bytes into one accumulator with no early exit and returns only bool"
    - "cap_verify builds CAP_LABEL || body in one fixed 32-byte stack buffer and defers the constant-time compare to crypto::mac::verify_keyed"
    - "zeroize replaces each Key/prefix with a zero value; Zeroizing scrubs the replaced bytes on the drop the assignment performs"

key-files:
  created:
    - crypto/tests/session.rs
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-03.md
  modified:
    - crypto/src/session.rs

key-decisions:
  - "Transcript is an unkeyed Blake3 wrapper: absorb is one update of a whole message, chain absorbs the previous 32-byte hash first, hash finalizes without consuming"
  - "zeroize is implemented by assigning a fresh zero Key/prefix, so the old bytes are scrubbed by Zeroizing on drop; no mutable byte accessor was added to Key"
  - "is_zeroized answers only a bool and folds all 68 bytes with no early exit, staying inside 04-02's ban on accessors that return or print key bytes or the prefix"
  - "cap_tag's label is the 16-byte literal r9 StatelessCap1 and rekey_confirm's is r9 rekey ack; both are private constants, and the tests type the labels as literals"
  - "DECISION P6 accepted off: crypto/Cargo.toml and Cargo.lock are unchanged, and the module doc section '# What zeroize covers' states the scrubbed four keys and two prefixes and the unscrubbed per-packet hasher and cipher state"
  - "ADVANCED pin lines land as separate docs commits after each GREEN commit, because a commit cannot record its own SHA"

patterns-established:
  - "A pin ledger records the KAT command verbatim and identifies which side (Python vs. the crate) is independent"
  - "A RED commit ships real signatures with wrong bodies so both RED failure lines are assertion failures, never compile errors"

requirements-completed: [WIRE-03, WIRE-05]

coverage:
  - id: D1
    description: "The transcript hash is unkeyed BLAKE3 over whole messages in wire order: A then B equals BLAKE3(A||B), B then A differs, empty equals BLAKE3(empty), and chain(prev) absorbs prev first"
    requirement: WIRE-05
    verification:
      - kind: integration
        ref: "crypto/tests/session.rs#session_transcript_is_blake3_of_messages_in_order"
        status: pass
      - kind: integration
        ref: "crypto/tests/session.rs#session_transcript_chains_across_rekeys"
        status: pass
      - kind: integration
        ref: "cargo xtask test (both crypto legs, rc 0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The six derivation outputs equal an independently computed Python HKDF vector on the default and scalar-backend legs, and a changed transcript bit or generation changes the keys"
    requirement: WIRE-05
    verification:
      - kind: unit
        ref: "crypto/src/session.rs#session::tests::session_derive_known_answer"
        status: pass
      - kind: unit
        ref: "crypto/src/session.rs#session::tests::session_nonce_is_prefix_then_sequence_le"
        status: pass
      - kind: integration
        ref: "crypto/tests/session.rs#session_transcript_bit_changes_keys"
        status: pass
      - kind: integration
        ref: "crypto/tests/session.rs#session_generation_changes_every_key"
        status: pass
      - kind: integration
        ref: "crypto/tests/session.rs#session_directions_differ"
        status: pass
      - kind: other
        ref: "grep -c '^KAT session-derive command: ' evidence/PINS-04-03.md == 1"
        status: pass
    human_judgment: false
  - id: D3
    description: "zeroize clears all four keys and both prefixes; is_zeroized answers false for any single non-zero byte in the 68 covered positions and true after c2s-only or pair teardown"
    requirement: WIRE-03
    verification:
      - kind: unit
        ref: "crypto/src/session.rs#session::tests::session_zeroize_clears_every_key_byte_and_prefix"
        status: pass
      - kind: unit
        ref: "crypto/src/session.rs#session::tests::session_is_zeroized_agrees_with_the_bytes"
        status: pass
      - kind: integration
        ref: "crypto/tests/session.rs#session_is_zeroized_through_the_public_api"
        status: pass
    human_judgment: false
  - id: D4
    description: "cap_tag is the first 8 bytes of keyed BLAKE3 over r9 StatelessCap1 || body under the domain Key; cap_verify accepts it and rejects body, tag or domain-key changes"
    requirement: WIRE-05
    verification:
      - kind: integration
        ref: "crypto/tests/session.rs#session_cap_tag_is_truncated_keyed_blake3_of_label_and_body"
        status: pass
      - kind: integration
        ref: "crypto/tests/session.rs#session_cap_tag_roundtrip_and_reject"
        status: pass
    human_judgment: false
  - id: D5
    description: "rekey_confirm is keyed BLAKE3 under the direction's MAC key over r9 rekey ack || generation LE; verify accepts the match and rejects other keys, generations and flipped bytes"
    requirement: WIRE-03
    verification:
      - kind: unit
        ref: "crypto/src/session.rs#session::tests::session_rekey_confirm_is_keyed_blake3_of_label_and_generation"
        status: pass
      - kind: integration
        ref: "crypto/tests/session.rs#session_rekey_confirm_binds_key_and_generation"
        status: pass
    human_judgment: false
  - id: D6
    description: "The packet MAC is keyed BLAKE3 under the Python-derived MAC key: split updates equal one shot and each flipped tag byte is rejected"
    requirement: WIRE-03
    verification:
      - kind: unit
        ref: "crypto/src/session.rs#session::tests::session_packet_mac_is_keyed_blake3_under_the_mac_key"
        status: pass
      - kind: integration
        ref: "crypto/tests/session.rs#session_packet_mac_split_updates_equal_one_shot"
        status: pass
      - kind: integration
        ref: "crypto/tests/session.rs#session_packet_mac_verify_rejects_each_flipped_tag_byte"
        status: pass
    human_judgment: false
  - id: D7
    description: "The module documentation states exactly what zeroize does and does not cover (P6 off): the four keys and two prefixes are scrubbed, the per-packet hasher and cipher state are not"
    requirement: WIRE-03
    verification: []
    human_judgment: true
    rationale: "The words must be read against the implementation to judge that they neither overclaim nor underclaim; a grep for the heading proves presence, not accuracy."

# Metrics
duration: 22min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 03: `crypto::session` complete — derivation vector, transcript, zeroize, capability tag, re-key confirm Summary

**`crypto::session` now pins the P2 derivation with an independent Python vector on both crypto legs, hashes the handshake with a chainable unkeyed BLAKE3 transcript, scrubs all four keys and two prefixes on teardown behind a non-leaking `is_zeroized`, and exposes the 8-byte `StatelessCap` tag and the re-key confirm as keyed BLAKE3 — with `crypto/`'s first in-crate test module.**

## Performance

- **Duration:** ~22 min
- **Started:** 2026-11-25T19:24Z (worktree spawn)
- **Completed:** 2026-11-25T19:46Z
- **Tasks:** 2
- **Files modified:** 3 (1 modified, 2 created)

## Accomplishments

- **Independent derivation vector.** One Python 3 command using only `hmac`, `hashlib` and `struct` computes the six outputs (two cipher keys, two MAC keys, two prefixes) for secret `0x00..=0x1f`, transcript `0x20..=0x3f`, generation 0. The command and output head the ledger's `KAT session-derive command:` line, and the in-crate test compares all six byte for byte — on both the default and scalar-backend legs (`session_derive_known_answer ... ok` twice under `cargo xtask test`). The tracer's `derive` matched the vector unchanged.
- **Transcript hash and order.** `Transcript` wraps an unkeyed BLAKE3 hasher: `absorb` is one whole-message update, `chain(prev)` absorbs the previous 32-byte hash first, `hash` finalizes without consuming. Pins show A-then-B equals `blake3_hash(A||B)`, B-then-A differs, empty equals BLAKE3 of the empty string, and a single flipped transcript bit moves the `c2s` MAC key for all 256 bits.
- **Teardown zeroization at the byte level.** `DirectionKeys::zeroize` clears the cipher key, MAC key and prefix; `SessionKeys::zeroize` both directions; `is_zeroized` folds all 68 covered bytes into one accumulator with no early exit and returns only a `bool`. Tests read the raw bytes (in-crate, where `Key::bytes` is visible) and cover each of the 68 single-non-zero-byte positions plus the c2s-only and pair cases.
- **Capability tag and re-key confirm as keyed BLAKE3.** `cap_tag` is the first 8 bytes of keyed BLAKE3 under the caller's domain `Key` over the 16-byte literal `r9 StatelessCap1` followed by the 16 body bytes; `cap_verify` goes through `crypto::mac::verify_keyed` (constant-time) over a 32-byte stack buffer. `rekey_confirm` is keyed BLAKE3 under the direction's MAC key over `r9 rekey ack` followed by the generation little-endian, and its verify rejects the other direction, another generation and a flipped byte.
- **`crypto/`'s first in-crate test module**, with a comment stating why: key bytes are unreadable outside the crate, so the raw-byte pins must live beside the private fields; public-behaviour pins stay in `crypto/tests/session.rs`, which imports `crypto::` only.
- **P6 outcome (accepted, off):** `crypto/Cargo.toml` and `Cargo.lock` are unchanged by this plan. The module doc's `# What zeroize covers` section states the scrubbed four keys and two prefixes and the unscrubbed per-packet keyed hasher and ChaCha20 cipher state, rather than overclaiming.

## Task Commits

Each task was committed atomically (RED, GREEN, pin advance):

1. **Task 1 RED: derivation vector and transcript pins** - `dfb90d9` (test)
2. **Task 1 GREEN: transcript hash and pinned derivation** - `d040974` (feat)
3. **Task 1 pin advance** - `b318fe1` (docs)
4. **Task 2 RED: teardown, capability tag and re-key confirm pins** - `b835861` (test)
5. **Task 2 GREEN: zeroize, capability tag and re-key confirm** - `559ce7b` (feat)
6. **Task 2 pin advance** - `9d702a9` (docs)

**Plan metadata:** the SUMMARY commit follows (docs).

_Note: the ADVANCED pin lines are separate docs commits because a commit cannot record its own SHA; this follows the 04-02 precedent._

## Files Created/Modified

- `crypto/src/session.rs` — `Transcript` (`new`, `chain`, `absorb`, `hash`); `SessionKeys::zeroize`/`is_zeroized`; `DirectionKeys::zeroize`/`is_zeroized`/`rekey_confirm`/`verify_rekey_confirm`; `CAP_BODY_LEN`/`CAP_TAG_LEN`, `cap_tag`, `cap_verify`; the module doc section `# What zeroize covers`; the in-crate `tests` module (6 pins).
- `crypto/tests/session.rs` — 11 public pins: direction/generation separation, 256-bit transcript sensitivity, transcript order and re-key chain, public zeroize query, packet-MAC split/verify, capability tag shape/round-trip, re-key confirm binding.
- `evidence/PINS-04-03.md` — the `KAT session-derive command:` line with its independence note and eight `PIN` lines (`session-derive` and `session-teardown`, RED/GREEN-1/GREEN-2/ADVANCED).

## Decisions Made

- **Transcript shape.** Unkeyed BLAKE3 (`crate::mac::Blake3::new`), absorbing each whole message; `chain` absorbs the previous hash as the first input. This is the P2/D-17 contract and needs no new primitive.
- **zeroize by reassignment.** `DirectionKeys::zeroize` assigns a fresh zero `Key`/`Zeroizing` prefix; each replaced `Key` and prefix is itself `Zeroizing`, so the old bytes are scrubbed on the drop the assignment performs. This avoided adding a mutable byte accessor to `Key` (which would sit against 04-02's ban on key-byte accessors).
- **`is_zeroized` is the only new query**, returns `bool`, and has no early return in its 68-byte fold. It exists so the transport plans (04-07, 04-08) can assert teardown, `failure` and each re-key cutover really called `zeroize` from outside the crate.
- **Labels are private constants.** `CAP_LABEL` (`r9 StatelessCap1`, 16 bytes) and `REKEY_LABEL` (`r9 rekey ack`, 12 bytes) are private; the three `..._is_keyed_blake3_...` tests type the labels as literals and call `crypto::mac::blake3_keyed` directly, so a wrong-but-self-consistent label, key or truncation would fail them.
- **P6 stays off** per DECISION P6; the limitation is documented in `# What zeroize covers` and no feature or lockfile line changed.

## Deviations from Plan

None — the plan executed as written, including its conditional branches. The plan's `files_modified` listed `crypto/src/lib.rs` (no change needed: `pub mod session;` already existed), and `crypto/Cargo.toml`/`Cargo.lock` (correctly unchanged because DECISION P6 is off).

## Issues Encountered

- **One new clippy advisory:** `clippy::new_without_default` fires on the new `Transcript::new`. It is advisory only (`cargo xtask clippy` exits 0) and matches the pre-existing, deliberately unsuppressed advisory on `crypto::mac::Blake3`, so it was left consistent with the project's choice rather than adding an `impl Default` the plan did not ask for.
- No failure required a fix attempt; every RED failure was the intended assertion failure on a stub.

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None. The RED stubs (zero-returning `Transcript::hash`, always-`true` `is_zeroized`, no-op `zeroize`, zeroed `cap_tag`/`rekey_confirm`, always-`Ok` verifies) were all replaced in the two GREEN commits; no stubbed body remains.

## Next Phase Readiness

- `crypto::session` now exposes everything `port` needs from a key: `derive`, keystream, packet MAC, `Transcript`, `zeroize`/`is_zeroized`, `cap_tag`/`cap_verify`, `rekey_confirm`/`verify_rekey_confirm`. Plans 04-07 and 04-08 can assert teardown through `is_zeroized` and use `rekey_confirm` for the cutover acknowledgement.
- The derivation `info`, transcript definition and nonce construction are pinned by independent vectors, so plan 04-13 can write the formulas into RFC section 3.2 and cite these tests.
- `cargo xtask test`, `cargo xtask check` and `cargo xtask clippy` all exit 0 at `9d702a9`; the `port` tracer tests still pass unchanged.
- Kernel bytes are not touched by this plan: only `crypto/` library code and tests changed, and nothing reachable from a booted kernel links differently in a way this plan introduced.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- Created files exist: `crypto/tests/session.rs`, `evidence/PINS-04-03.md`, this SUMMARY.
- All six commits are ancestors of HEAD: `dfb90d9`, `d040974`, `b318fe1`, `b835861`, `559ce7b`, `9d702a9`.
- Measured commit count from `plan_head_before` `f2455ef` is 6, matching the `actuals.commits` field.
- The ledger greps pass: `^KAT session-derive command: ` == 1 and `^PIN (session-derive|session-teardown) (RED|GREEN-1|GREEN-2|ADVANCED)` == 8.
- `cargo test -p crypto --lib session` 6 passed, `--test session` 11 passed, `cargo test -p port --test aead_transport` 2 passed, `cargo xtask test` rc 0 twice, `cargo xtask check` rc 0.
