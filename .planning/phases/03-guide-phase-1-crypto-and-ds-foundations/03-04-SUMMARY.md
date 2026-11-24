---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 04
subsystem: crypto
tags: [crypto, blake3, keyed-mac, key-hierarchy, hkdf-expand, typestate, zeroize, no_std, pin-ledger]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "crypto/ workspace crate, HKDF-SHA256 hkdf_expand and the two crypto test legs (@03-03)"
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "crypto+ds host test legs, the crypto scalar-backend leg and the crypto --doc leg (@03-12)"
provides:
  - "crypto::mac: blake3_hash, blake3_keyed, the incremental Blake3 hasher and verify_keyed with constant-time full and truncated tag verification"
  - "the official BLAKE3 vector file committed byte for byte, with 35-case pins for hash and keyed_hash, one-shot and incremental, on both test legs"
  - "crypto::key: Key over Zeroizing<[u8; 32]>, k_file with the fixed 12-byte little-endian Qid encoding, and derive_labelled for transient and session keys"
  - "KeyCapability<ReadWrite|ReadOnly>: seal/verify/apply_keystream, read_only(), and the type-level read-only guarantee proven by a compile_fail doctest pair"
affects: [03-09, phase-4, phase-5]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 16899
  tasks: 3
  commits: 9
  plan_head_before: 7e27cc7186b5eb114df0c381ca3e57827f30d48d
  plan_head_after: c95e3aff044a0c207c924ce1a86d17032c54cbee

tech-stack:
  added: []
  patterns:
    - "Thin wrapper: no upstream type appears in a crypto/ public signature — mac and key take and return slices, fixed arrays and r9 types (D-18)"
    - "Truncated MAC verification by a no-early-exit XOR fold; full 32-byte tags through blake3::Hash's constant-time PartialEq"
    - "Typestate capability: KeyCapability<S> over private state structs; ReadOnly has no seal, proven by a compile_fail doctest with a passing twin"
    - "Zeroizing key material with no Debug/Display and no public byte accessor; tests compare keys through ct_eq"
    - "IG-V01: committed RED pin, GREEN twice, ADVANCED commit in evidence/PINS-03-04.md"

key-files:
  created:
    - crypto/src/mac.rs
    - crypto/src/key.rs
    - crypto/tests/blake3_vectors.rs
    - crypto/tests/vectors/blake3_test_vectors.json
    - crypto/tests/keys.rs
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-04.md
  modified:
    - crypto/src/lib.rs

key-decisions:
  - "verify_keyed accepts tags of 1..=32 bytes: a 32-byte tag compares through blake3::Hash's constant-time PartialEq (constant_time_eq_32); a shorter tag folds the XOR of every compared byte into one accumulator with no early exit and tests it once at the end"
  - "Blake3 wraps blake3::Hasher in a private field; no public item names a blake3:: type, so the pinned crate stays an implementation detail (D-18)"
  - "k_file builds a fixed 12-byte info — qid_path as 8 bytes little-endian then qid_vers as 4 bytes little-endian (RESEARCH Open Question 4) — and expands it under K_user; the encoding is pinned by an independently computed KAT"
  - "derive_labelled uses Hkdf::expand_multi_info(&[label, context]): crypto/ is no_std with no allocator, the RFC's info is one opaque string, and the crate documents that feeding the components in sequence is feeding their concatenation"
  - "Key derives Clone only and holds Zeroizing<[u8; KEY_LEN]>; its byte accessor is pub(crate), ct_eq is the only public comparison, and no public method returns key bytes"
  - "The read-only guarantee is type-level, not cryptographic: keyed BLAKE3 is symmetric, so ReadOnly's blake3_verify can compute a tag; the module docs say so in plain words and the compile_fail doctest proves no seal method exists"
  - "read_only() clones k_dec unchanged (IG-P01) and sets blake3_verify from the parent's k_mac, keeping the cipher and MAC keys separate (T-03-18)"
  - "The Key newtype (from_bytes, ct_eq) is complete at its RED commit because it is the comparison mechanism, not the behaviour under test; only k_file and derive_labelled returned zero keys at RED"

requirements-completed: [CRYP-03, CRYP-04]

coverage:
  - id: D1
    description: "BLAKE3 hash and keyed_hash pass all 35 cases of the official vector file (32-byte output), one-shot and incremental split at the midpoint, on the default and scalar-backend legs"
    requirement: CRYP-03
    verification:
      - kind: unit
        ref: "crypto/tests/blake3_vectors.rs#blake3_pin_official_hash_all_cases, blake3_pin_official_keyed_hash_all_cases, blake3_incremental_matches_one_shot (5 passed per leg; `cargo xtask test` shows `test blake3_pin_official_keyed_hash_all_cases ... ok` twice)"
        status: pass
    human_judgment: false
  - id: D2
    description: "verify_keyed accepts a full 32-byte tag and its 8-byte prefix, rejects every single-bit flip of either, and reports a 0- or 33-byte tag as BadTagLength"
    requirement: CRYP-03
    verification:
      - kind: unit
        ref: "crypto/tests/blake3_vectors.rs#blake3_keyed_verify_accepts_and_rejects"
        status: pass
    human_judgment: false
  - id: D3
    description: "K_file matches an independently computed known answer; path/vers adjacency and one-bit changes give different keys; zero/empty inputs work and differ from the parent; the unframed labelled concatenation and label/context order are pinned"
    requirement: CRYP-04
    verification:
      - kind: unit
        ref: "crypto/tests/keys.rs#k_file_known_answer, k_file_binds_path_and_vers, k_file_and_derive_accept_zero_and_empty, derive_labelled_concatenation_is_pinned (8 passed per leg)"
        status: pass
      - kind: other
        ref: "evidence/PINS-03-04.md `KAT k_file` line: Python hmac+hashlib HKDF-Expand output ce90009a...0afe equals the test literal"
        status: pass
    human_judgment: false
  - id: D4
    description: "KeyCapability<ReadWrite> seals and verifies, the ReadOnly sibling verifies the parent's tag, the sibling keeps the parent's k_dec, and a tag under k_mac does not verify under k_dec; ReadOnly has no seal (compile_fail doctest with a passing twin)"
    requirement: CRYP-04
    verification:
      - kind: unit
        ref: "crypto/tests/keys.rs#read_write_seals_and_verifies, read_only_verifies_what_read_write_sealed, read_only_sibling_keeps_parent_k_dec, mac_key_is_separate_from_cipher_key"
        status: pass
      - kind: unit
        ref: "cargo test -p crypto --doc: key::KeyCapability<ReadOnly> verify example ok and the compile_fail twin ok"
        status: pass
    human_judgment: false
  - id: D5
    description: "The limit of the read-only guarantee is stated in plain words as type-level rather than cryptographic, and the key hierarchy lives in crypto/ with no port/src edit (D-22)"
    requirement: CRYP-04
    verification:
      - kind: other
        ref: "crypto/src/key.rs module doc sections 'The read-only guarantee is type-level, not cryptographic'; `git diff 7e27cc7 HEAD -- port/src` empty; no Cargo.lock change"
        status: pass
    human_judgment: true
    rationale: "That the wording is plain and does not overclaim is a reading of the module documentation; the mechanical halves are the compile_fail doctest and the empty port/src diff"

# Metrics
duration: 2min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 04: Keyed BLAKE3 as the only MAC; `K_file`, the labelled expand and the `KeyCapability` typestate

**CRYP-03 and CRYP-04 are complete: keyed BLAKE3 behind `crypto::mac` passes all 35 official cases for hash, keyed hash and the keyed tree mode with constant-time truncated-tag verification, and `crypto::key` holds the zeroizing `Key`, the KAT-pinned `K_file`, the unframed labelled expand and the `KeyCapability<ReadWrite|ReadOnly>` typestate whose read-only limit is stated as type-level.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-11-24T14:36:38Z
- **Completed:** 2026-11-24T14:39:25Z
- **Tasks:** 3
- **Files modified:** 7 (6 created, 1 modified)

## Accomplishments

- `crypto::mac` provides `blake3_hash`, `blake3_keyed`, the incremental `Blake3` (`new`, `new_keyed`, `update`, `finalize`) and `verify_keyed`; the upstream `blake3` types never appear in a public signature (D-18). The module doc states D-05 (keyed BLAKE3 is the only MAC; Poly1305 is not implemented) and D-07 ("HMAC-BLAKE3" is this keyed mode truncated; no HMAC is built over BLAKE3), and explains that tree mode needs no separate function.
- The official `test_vectors.json` is committed byte for byte: 31,922 bytes, sha256 `dcb91ea8…f624`, upstream tag `1.8.7`. The test asserts 35 cases and the documented key, and pins `hash` and `keyed_hash` for every case, one-shot and incremental (split at each case's midpoint), on both the default and scalar-backend legs. Cases from 1025 bytes up exercise tree mode and keyed tree mode.
- `verify_keyed` accepts the full 32-byte tag and its 8-byte prefix (the RFC's truncated capability tag), rejects all 64 single-bit flips of the 8-byte tag and all 256 of the full tag with `Err(TagMismatch)`, and returns `Err(BadTagLength)` for a 0- or 33-byte tag.
- `crypto::key::k_file` is `HKDF-Expand(K_user, Qid.path || Qid.vers, 32)` with a fixed 12-byte info (8-byte little-endian path, then 4-byte little-endian vers). It matches an independently computed known answer (`ce90009a…0afe`), and the adjacency cases, every one-bit path change and every one-bit vers change give different keys.
- `derive_labelled` implements the RFC's unframed `label || context` info; the pins record both that `("ab","c")` equals `("a","bc")` and that `("a","b")` differs from `("b","a")`, so a later framing change is a visible pin advance. Zero and empty inputs are deterministic and distinct from the parent.
- `Key` holds `Zeroizing<[u8; KEY_LEN]>`, derives `Clone` only, has no `Debug`/`Display` and no public byte accessor; `ct_eq` is a no-early-exit fold.
- `KeyCapability<ReadWrite>` seals under `k_mac` and verifies; `read_only()` keeps the parent's `k_dec` and sets `blake3_verify` from `k_mac`; `apply_keystream` runs `chacha20_xor` under `k_dec`. The ReadOnly type has no `seal`, no public constructor other than `read_only()`, and no key accessor. The doctest pair differs only in its final call: the passing twin proves the example compiles, the `compile_fail` twin proves `seal` does not exist on ReadOnly.
- The module doc's "The read-only guarantee is type-level, not cryptographic" section states plainly that keyed BLAKE3 is symmetric, that the verification key can compute a tag, and that the guarantee is the type (no method, private fields, no accessor).
- `nix develop -c cargo xtask test` (twice), `cargo xtask check` and `cargo xtask clippy` all exit 0. `Cargo.lock` and `port/src` are unchanged; no dependency was added (T-03-SC), and D-21's kernel-byte stop does not trigger.

## Task Commits

Each task was committed atomically, RED before GREEN:

1. **Task 1, RED: BLAKE3 surface and the official vector pins** — `62dee43` (test)
2. **Task 1, GREEN: BLAKE3 hash and keyed MAC implementation** — `83e5b26` (feat)
3. **Task 1, pin advance** — `28764cc` (evidence)
4. **Task 2, RED: key hierarchy surface and the KAT** — `0b6d542` (test)
5. **Task 2, GREEN: `k_file` and `derive_labelled`** — `b10c7e0` (feat)
6. **Task 2, pin advance** — `4eee8ec` (evidence)
7. **Task 3, RED: typestate surface and the four capability tests** — `c692d68` (test)
8. **Task 3, GREEN: `KeyCapability` and the type-level read-only guarantee** — `0101a90` (feat)
9. **Task 3, pin advance** — `c95e3af` (evidence)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `crypto/src/mac.rs` — the BLAKE3 module: `BLAKE3_OUT_LEN`, `BLAKE3_KEY_LEN`, `blake3_hash`, `blake3_keyed`, `Blake3`, `verify_keyed`; D-05/D-07 module doc and the truncated-tag rule
- `crypto/src/key.rs` — `KEY_LEN`, `Key` (`from_bytes`, `ct_eq`, `pub(crate) bytes`), `k_file`, `derive_labelled`, `ReadWrite`, `ReadOnly`, `KeyCapability<S>`; D-22 landing-site note and the type-level guarantee section with the doctest pair
- `crypto/src/lib.rs` — `pub mod key;` and `pub mod mac;`
- `crypto/tests/blake3_vectors.rs` — the fixed-shape extractor over the committed fixture and the five BLAKE3 pins
- `crypto/tests/vectors/blake3_test_vectors.json` — the official file, byte for byte (tag 1.8.7)
- `crypto/tests/keys.rs` — the KAT, binding, zero/empty and ordering pins plus the four typestate tests
- `evidence/PINS-03-04.md` — `VECTORS blake3`, `KAT k_file`, and RED/GREEN-1/GREEN-2/ADVANCED for `blake3-official-vectors`, `key-hierarchy` and `key-capability`

## Decisions Made

- `verify_keyed` splits its two paths: the 32-byte tag uses `blake3::Hash`'s `PartialEq` (constant-time `constant_time_eq_32`), and a shorter tag uses an explicit no-early-exit XOR fold tested once at the end. Neither path compares two byte slices with `==`; a 0- or >32-byte tag is refused before any comparison.
- `Blake3` wraps `blake3::Hasher` in a private field, so the incremental API exposes no upstream type (D-18).
- The BLAKE3 test's extractor is fixed-shape (string and integer fields plus a `{`/`}` split) rather than a JSON crate: one file, one shape, no new dependency (03-RESEARCH.md F4). The case count is asserted as 35 so a truncated file cannot pass.
- `k_file` uses the fixed 12-byte little-endian encoding chosen in 03-RESEARCH.md Open Question 4; the test cites the independent Python computation recorded in the ledger.
- `derive_labelled` calls `Hkdf::expand_multi_info(&[label, context])` rather than concatenating into a buffer: `crypto/` is `no_std` with no allocator, and the pinned crate documents that feeding the components in sequence is equivalent to feeding their concatenation. The pinned `("ab","c") == ("a","bc")` test makes the choice observable.
- `read_only()` clones `k_dec` and moves the MAC key into `blake3_verify`, so the read-only sibling decrypts with the parent's cipher key (IG-P01) and verifies with a key that is separate from `k_dec` (T-03-18).
- The doctest pair lives on the `impl KeyCapability<ReadOnly>` block: a passing example seals on a `ReadWrite` capability, takes `read_only()` and verifies; the `compile_fail` twin is the same text with the last call replaced by `ro.seal(...)`.
- `K_file`'s `info` construction and `derive_labelled`'s 32-byte output use `expect` for the impossible HKDF error, with messages naming the RFC 5869 rules that make the failure unreachable.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The plan-commit ledger sentinel (`$(git rev-parse --git-dir)/gsd-plan-head-before-03-04`) held `11196de8`, a real but much older ancestor, before the first commit — the same stale-sentinel condition plan 03-03 hit. It was re-anchored to the actual plan start `7e27cc7` (the parent of the first RED commit) before the summary, so the measured `commits` count is 9.
- `cargo xtask clippy` (rc 0) reports one new advisory on the new code: `clippy::new_without_default` for `Blake3::new`. It was left as the plan fixes the public surface (`new`, `new_keyed`, `update`, `finalize`); adding `impl Default` would be an unrequested public API addition. The gate is exit 0.

## Known Stubs

None. Every RED skeleton was replaced by the GREEN implementation in the same task; no placeholder values, TODOs or unwired data sources remain in the plan's files.

## Threat Flags

None beyond the plan's `<threat_model>`: T-03-13 is mitigated by `blake3::Hash` equality for 32-byte tags and the no-early-exit fold for truncated tags; T-03-14 by `Zeroizing`, no `Debug`/`Display` and the `pub(crate)` byte accessor; T-03-15 by the typestate, the private fields and the `compile_fail` doctest with its passing twin (residual symmetric-MAC risk documented and flagged below); T-03-16 by the sha256 gate and the 35-case assertion; T-03-17 by the fixed 12-byte `info`, the documented unframed expand and the deferred W4 session layout; T-03-18 by separate `k_dec`/`k_mac` fields and `mac_key_is_separate_from_cipher_key`; T-03-SC by the unchanged `Cargo.lock`.

## For owner review

- **The read-only guarantee is type-level, not cryptographic** (orchestrator decision 2; 03-RESEARCH.md C6 and assumption A5). Keyed BLAKE3 is symmetric, so the key `KeyCapability<ReadOnly>` holds as `blake3_verify` can compute a valid tag as well as check one. What prevents sealing is the type: `KeyCapability<ReadOnly>` has no `seal` method, its fields are private and no accessor returns key bytes. A cryptographic guarantee would require a verification value other than the MAC key, which is a design change outside this plan. The module docs and the doctest pair state this plainly.
- **The `K_file` encoding** (`Qid.path` as 8 bytes little-endian then `Qid.vers` as 4 bytes little-endian, 12 bytes total) becomes part of every derived file key. It is pinned by an independently computed known answer, and plan 03-09 records the choice (RESEARCH Open Question 4) in PROJECT.md.
- **CRYP-03 edge coverage** (the plan's flagged `unclassified` probe row): this plan treats the official file's own lengths (0–8, the 63/64/65, 1023/1024/1025 and later chunk boundaries, up to 102,400 bytes) as the hash edge coverage, and the truncated-tag tests as the MAC edge coverage. The owner should confirm at verification that no other edge was meant.
- The pinned `zeroize =1.9.2` remains flagged from plan 03-01/03-09 for the owner's review at phase verification.

## Next Phase Readiness

- CRYP-03 and CRYP-04 are complete on both the host default backend and the scalar backend: the MAC of the Phase 4 wire format and the Phase 5 page tags exists and is pinned, and Phase 5's `port/src/segcrypt.rs` can import `Key`, `KeyCapability`, `ReadWrite` and `ReadOnly` instead of defining them (D-22).
- No `port/src` edit, no `Cargo.lock` change and no dependency added, so D-21's kernel-byte stop condition remains unmet and the phase continues.
- Plan 03-09 can copy the `K_file` encoding and the type-level guarantee into PROJECT.md as planning-level rows for owner review at Phase 3 verification.

---

*Phase: 03-guide-phase-1-crypto-and-ds-foundations*
*Completed: 2026-11-24*

## Self-Check: PASSED

All six created files exist on disk and all nine task commits are ancestors of HEAD. Plan-level verification at the final state: the vector file's sha256 gate prints 1, `cargo test -p crypto --test blake3_vectors` reports 5 passed, `cargo test -p crypto --test keys` reports 8 passed, `cargo test -p crypto --doc` runs the passing example and the `compile fail ... ok` twin, and `cargo xtask test` (twice), `check` and `clippy` all exit 0. `git diff 7e27cc7 HEAD -- port/src Cargo.lock` is empty, and the ledger holds 12 `^PIN` lines (RED, GREEN-1, GREEN-2, ADVANCED for each of the three pin ids) plus `VECTORS blake3` and `KAT k_file`.
