---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 03
subsystem: crypto
tags: [crypto, hkdf-sha256, hmac-sha256, chacha20, rfc-5869, rfc-4231, rfc-8439, counter-overflow, no_std, pin-ledger]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "crypto/ workspace crate, the six exact pins, hmac_sha256 and the two crypto test legs (@03-01)"
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "the static lint contract gate over crypto/src run before every cargo leg (@03-02)"
provides:
  - "crypto::kdf::hkdf_extract and hkdf_expand with HKDF_MAX_OUTPUT (8160), on HMAC-SHA256"
  - "RFC 4231 cases 1 to 7 and RFC 5869 cases 1 to 3 pinned through crypto's API on the default and scalar-backend legs"
  - "crypto::page: chacha20_block and chacha20_xor with the RFC 8439 32-bit counter and 96-bit nonce, plus the four length/limit constants"
  - "RFC 8439 2.3.2 and 2.4.2 pins and the counter-overflow boundary pinned on both test legs"
  - "the per-key-and-nonce ChaCha20 limit stated in the module doc and recorded as a BOUNDARY ledger line for owner review"
affects: [03-04, phase-4, phase-5, phase-6]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 7569
  tasks: 2
  commits: 6
  plan_head_before: 271f1db1125a4074d982b6d23e047216b86e2ad3
  plan_head_after: 7feb35cac7434a2417080ad985664f6019e20e28

tech-stack:
  added: []
  patterns:
    - "Thin wrapper: hkdf/hmac/sha2/chacha20 never appear in a crypto/ public signature (D-18)"
    - "Restated crate boundary: r9 computes the ChaCha20 block budget itself before any byte is written, so the pinned crate's limit cannot move silently"
    - "IG-V01: committed RED pin, GREEN twice on both test legs, ADVANCED commit in evidence/PINS-03-03.md"

key-files:
  created:
    - crypto/src/page.rs
    - crypto/tests/rfc5869.rs
    - crypto/tests/rfc8439.rs
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-03.md
  modified:
    - crypto/src/kdf.rs
    - crypto/src/lib.rs
    - crypto/tests/rfc4231.rs

key-decisions:
  - "hkdf_extract wraps Hkdf::<Sha256>::extract and returns the 32-byte PRK; hkdf_expand checks the RFC 5869 255 x 32 length rule before touching the output, then wraps from_prk + expand with no fallback path"
  - "Hkdf::from_prk cannot fail in hkdf_expand because the PRK parameter is exactly [u8; 32]; the expect documents the type-level guarantee rather than an unchecked runtime condition"
  - "chacha20_xor computes ceil(len/64) and compares it with CHACHA20_MAX_BLOCKS - counter before the cipher is built, restating the pinned crate's remaining-blocks rule in r9's own code (T-03-09)"
  - "The last usable block index is u32::MAX - 1 (2^32 - 1 blocks, 274,877,906,880 bytes from counter 0), one 64-byte block fewer than RFC 8439; block index u32::MAX is never produced and the module doc states the limit"
  - "chacha20_block is chacha20_xor over 64 zero bytes, so both functions share one boundary rule and one error path"
  - "The RFC 8439 2.4.2 expected ciphertext was corrected against the fetched RFC text after a manual transcription slip; the implementation had matched the RFC"

requirements-completed: [CRYP-01, CRYP-02]

coverage:
  - id: D1
    description: "HMAC-SHA256 passes RFC 4231 test cases 1 to 7 through crypto::kdf on the default and scalar-backend legs"
    requirement: CRYP-01
    verification:
      - kind: unit
        ref: "crypto/tests/rfc4231.rs#hmac_sha256_pin_rfc4231_case1..case7 (7 ok lines per leg, 14 in `cargo xtask test`)"
        status: pass
    human_judgment: false
  - id: D2
    description: "HKDF-SHA256 passes RFC 5869 cases 1 to 3 (PRK and OKM), the 8160/8161-byte limit, the argument-order rule and the empty-input edges on both legs"
    requirement: CRYP-01
    verification:
      - kind: unit
        ref: "crypto/tests/rfc5869.rs#hkdf_sha256_pin_rfc5869_case1..case3, hkdf_expand_length_limits, hkdf_argument_order_is_pinned, hmac_sha256_accepts_empty_key_and_message (6 ok per leg)"
        status: pass
    human_judgment: false
  - id: D3
    description: "HKDF is built on HMAC-SHA256 from the pinned hkdf/hmac/sha2 crates; no XOR fold or non-RFC derivation exists in crypto/, and no upstream type appears in a public signature"
    requirement: CRYP-01
    verification: []
    human_judgment: true
    rationale: "Absence of a fallback derivation and of upstream types in public signatures is a reading of the module, not a test assertion; the mechanical check is that every public item names only slices, arrays and crate::Result"
  - id: D4
    description: "ChaCha20 passes RFC 8439 2.3.2 and 2.4.2 plus the keystream-prefix, split-counter, overflow and counter-selection pins on both legs"
    requirement: CRYP-02
    verification:
      - kind: unit
        ref: "crypto/tests/rfc8439.rs#chacha20_pin_rfc8439_2_3_2, chacha20_pin_rfc8439_2_4_2, chacha20_block_boundaries, chacha20_counter_overflow_is_an_error, chacha20_last_block_constant_matches, chacha20_counter_selects_the_block (6 ok per leg)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The per-key-and-nonce limit is stated in the crypto::page module doc (last block index u32::MAX - 1, 274,877,906,880 bytes, one block fewer than RFC 8439) and measured into the ledger"
    requirement: CRYP-02
    verification:
      - kind: other
        ref: "crypto/src/page.rs#Limit per key and nonce; evidence/PINS-03-03.md BOUNDARY chacha20 last-block=4294967294 max-bytes=274877906880; `cargo xtask check` rc 0 on both kernel specs"
        status: pass
    human_judgment: false
  - id: D6
    description: "Each requirement had a committed RED pin before its implementation and went GREEN twice (D-02, IG-V01)"
    requirement: CRYP-01
    verification:
      - kind: other
        ref: "evidence/PINS-03-03.md: 8 ^PIN lines (RED, GREEN-1, GREEN-2, ADVANCED for hkdf-sha256-rfc5869 and chacha20-rfc8439), plus PIN hmac-sha256-rfc4231 GREEN"
        status: pass
    human_judgment: false

# Metrics
duration: 2min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 03: HMAC-SHA256 and HKDF-SHA256 complete; ChaCha20 with the RFC 8439 layout

**CRYP-01 and CRYP-02 are complete: HKDF-SHA256 on HMAC-SHA256 passes every RFC 5869 and RFC 4231 case on both test legs, and ChaCha20 behind `crypto::page` passes the RFC 8439 block and encryption vectors with the last usable block index `u32::MAX - 1` pinned and documented.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-11-24T14:33:15Z
- **Completed:** 2026-11-24T14:35:27Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- `crypto::kdf::hkdf_extract(salt, ikm) -> [u8; 32]` and `hkdf_expand(prk, info, okm) -> Result<()>` are thin wrappers over `hkdf =0.13.0` on `sha2 =0.11.1` and `hmac =0.13.0`; no upstream type appears in a public signature (D-18). `HKDF_MAX_OUTPUT` is 8160 (255 × 32), and a longer request returns `Err(Error::OutputTooLong)` before the output buffer is touched (T-03-11).
- RFC 4231 §4.2 to §4.8 test cases 1 to 7 and RFC 5869 §A.1 to §A.3 test cases 1 to 3, every value copied from the RFC text at execution time, pass through `crypto`'s API on the host default backend and on the scalar backend the kernels select. `hkdf_sha256_pin_rfc5869_case3 ... ok` appears twice in `cargo xtask test`; the seven RFC 4231 pins appear fourteen times across the two legs.
- The HKDF edge rules are pinned: 8160 bytes `Ok`, 8161 bytes `Err(OutputTooLong)` with the buffer still holding its initial fill, zero-length output `Ok`, swapped extract arguments differing, `info` order `a||b` differing from `b||a`, and an empty key/message HMAC returning 32 bytes.
- `crypto::page` provides `chacha20_block` and `chacha20_xor` with the RFC 8439 layout: a caller-supplied 32-bit block counter and 96-bit nonce, never a default or generated value (D-06, T-03-10). The 8-byte-wire-nonce-to-12-byte-nonce rule is explicitly Phase 4's.
- RFC 8439 §2.3.2 (block function, counter 1) and §2.4.2 (114-byte "sunscreen" ciphertext plus the decrypt round trip) pass on both legs, along with the keystream-prefix test at 63/64/65 bytes, the split-at-64-bytes test with counters `n` and `n+1`, the counter-selection test and the counter-overflow boundary.
- The last-block boundary is pinned: `u32::MAX - 1` with 64 bytes `Ok` and 65 bytes `Err(CounterOverflow)`; `u32::MAX - 2` with 128 bytes `Ok` and 129 bytes the error; `u32::MAX` refuses 1 and 64 bytes, accepts a zero-length buffer, and `chacha20_block` returns the same error. Every refusal leaves the buffer untouched.
- `nix develop -c cargo xtask check` exits 0, so both kernel-spec legs compile the new module. `Cargo.lock` and `port/src` are unchanged by this plan; no dependency was added (T-03-SC).

## Task Commits

Each task was committed atomically, RED before GREEN:

1. **Task 1, RED: HKDF surface and the RFC 4231 / RFC 5869 pins** — `35abd75` (test)
2. **Task 1, GREEN: HKDF-SHA256 implementation** — `abef751` (feat)
3. **Task 1, pin advance** — `d88a2ba` (evidence)
4. **Task 2, RED: ChaCha20 module and the RFC 8439 pins** — `f1acfca` (test)
5. **Task 2, GREEN: ChaCha20 implementation** — `e87a29e` (feat)
6. **Task 2, pin advance** — `7feb35c` (evidence)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `crypto/src/kdf.rs` — `HKDF_MAX_OUTPUT`, `hkdf_extract`, `hkdf_expand`; module doc now lists the pinned standards (RFC 4231 §4.2-4.8, RFC 5869 §A.1-A.3) and the ledger
- `crypto/src/page.rs` — the ChaCha20 module: RFC 8439 layout and caller duty, the `# Limit per key and nonce` section, `CHACHA20_KEY_LEN`/`NONCE_LEN`/`BLOCK_LEN`/`MAX_BLOCKS`, `chacha20_block`, `chacha20_xor` built on `try_seek` and `try_apply_keystream`
- `crypto/src/lib.rs` — `pub mod page;`
- `crypto/tests/rfc4231.rs` — RFC 4231 cases 2 to 7 added beside case 1; every test carries the RFC section and case in its doc comment
- `crypto/tests/rfc5869.rs` — RFC 5869 cases 1 to 3, the length limits, the argument-order pin and the empty-input HMAC pin
- `crypto/tests/rfc8439.rs` — RFC 8439 §2.3.2 and §2.4.2, the keystream boundaries, the overflow boundary, the constant check and the counter-selection pin
- `evidence/PINS-03-03.md` — RED, GREEN-1, GREEN-2 and ADVANCED for `hkdf-sha256-rfc5869` and `chacha20-rfc8439`, `PIN hmac-sha256-rfc4231 GREEN`, and the `BOUNDARY chacha20` line

## Decisions Made

- `hkdf_expand` enforces the RFC 5869 §2.3 length rule itself, before calling the crate, so an over-long request returns `OutputTooLong` with the output untouched on every path; there is no fallback derivation.
- `Hkdf::from_prk` is called with `expect` because the `&[u8; 32]` parameter makes the crate's only error unreachable; the comment states the type-level guarantee rather than pretending a runtime branch exists.
- `chacha20_xor` recomputes the block budget (`ceil(len/64)` against `CHACHA20_MAX_BLOCKS - counter`) in r9's own code before the cipher is constructed, so the per-key limit does not move silently with a crate update, and an overflow writes nothing (T-03-09).
- The module doc and `CHACHA20_MAX_BLOCKS` state the limit as 2^32 - 1 blocks, 274,877,906,880 bytes from counter 0, block indices 0 to `u32::MAX - 1`, one 64-byte block fewer than RFC 8439 allows; the reason is the pinned `chacha20 =0.10.2` crate's refusal of the last counter value.
- `chacha20_block` is `chacha20_xor` over 64 zero bytes, keeping one boundary rule and one error path for both functions.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The first transcription of the RFC 8439 §2.4.2 expected ciphertext had a one-character slip near the end (`...8edf2785e428744d` instead of `...8eedf2785e42874d`). The GREEN run surfaced it: the implementation's output matched the RFC text fetched into `/tmp/rfc8439.txt` and differed from the test literal. The expected string was corrected from the RFC text (not from the implementation) and all six pins then passed. This is exactly the failure mode Pitfall 10 warns about; the fetched RFC text was the arbiter.
- The plan-commit ledger sentinel held a base commit (`91d2237`) that does not exist in this repository (a stale file from an earlier checkout). It was re-anchored to the actual plan start `271f1db` before the first task commit, so the measured `commits` count below is 6.

## Known Stubs

None. The RED skeletons were replaced by the GREEN implementations in the same task; no placeholder values, TODOs or unwired data sources remain in the plan's files.

## Threat Flags

None beyond the plan's `<threat_model>`: T-03-09 is mitigated by the pre-write block-budget check and the pinned boundary test on both legs; T-03-10 is transferred by the explicit nonce/counter API and the documented caller duty; T-03-11 by the length check before the output is touched; T-03-12 by RFC-copied vectors on both legs; T-03-SC by the unchanged `Cargo.lock`.

## For owner review

The ChaCha20 limit per key and nonce, measured and now recorded for plan 03-09 to copy into PROJECT.md:

- Last usable block index: `u32::MAX - 1` (4294967294). Block index `u32::MAX` is never produced.
- Maximum bytes from counter 0: 274,877,906,880 (2^32 - 1 blocks × 64 bytes).
- RFC 8439 allows one 64-byte block more (2^32 blocks); the difference is the pinned `chacha20 =0.10.2` crate, which refuses the last counter value so its counter can never wrap.
- Ledger line: `BOUNDARY chacha20 last-block=4294967294 max-bytes=274877906880`

## Next Phase Readiness

- CRYP-01 and CRYP-02 are complete on both the host default backend and the scalar backend: the KDF and cipher substrate that plan 03-04 (key hierarchy), Phase 4 (the wire) and Phase 5 (page sealing) call through `crypto/` is pinned and green.
- `hkdf_expand` accepts a caller-supplied PRK, so plan 03-04 can build `derive_labelled` and the `K_file` binding on `hkdf_extract`/`hkdf_expand` without touching this module's API.
- `chacha20_xor` takes the counter explicitly; Phase 4 owns the rule that maps the 8-byte wire sequence nonce onto the 12-byte nonce, as the module doc states.
- No dependency, `Cargo.lock`, `port/src` or kernel-path change was made, so D-21's stop condition remains unmet and the phase continues.

---

*Phase: 03-guide-phase-1-crypto-and-ds-foundations*
*Completed: 2026-11-24*

## Self-Check: PASSED

All five claimed files exist on disk and all six task commits are ancestors of HEAD. `gsd_run check evaluation-scope --plan 03-03 --commits-only` resolved the six plan commits (plus one pre-existing 2026-11-01 commit whose subject happens to carry the same `03-03` prefix). Plan-level verification at the final state: `cargo xtask test` rc 0 twice with the RFC 5869 and RFC 8439 pins passing on both test legs, `cargo xtask check` rc 0, `Cargo.lock` and `port/src` unchanged since `271f1db`, and the ledger holds 8 `^PIN` lines plus the `BOUNDARY chacha20` line.
