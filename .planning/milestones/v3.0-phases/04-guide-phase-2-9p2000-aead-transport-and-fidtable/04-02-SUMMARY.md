---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "02"
subsystem: kernel-transport-crypto
tags: [9p2000-aead, hkdf, chacha20, blake3, packet-mac, lint-contract, tracer]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "crypto::key::Key, crypto::kdf (hkdf_extract/expand), crypto::mac (Blake3 keyed/incremental, verify_keyed), crypto::page (chacha20_xor); ds crate for the edge"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-01)
    provides: "the eight owner-accepted wire decisions (P1 to P8), the IG-D11 answer, the kernel byte baseline before.sha256"
provides:
  - "port depends on crypto and ds by path; lint rule R4 permits the edge for port only"
  - "crypto::session — derive, DirectionKeys, SessionKeys, PacketMac, DirectionKeys::mac/keystream"
  - "port::chan — HEADER_LEN/MAC_LEN/VERSION_AEAD, the eight header offsets, seal_packet, open_packet"
  - "the tracer pin: one sealed packet round-trips under cargo xtask test"
affects: [04-17 test-9p, port fid table, re-key, devmnt negotiate branch, phase-05 page sealing]

actuals:
  tokens: 7191
  tasks: 2
  commits: 4
  plan_head_before: b51e0650a3c0f192026d3b479c1ba66642cf3228
  plan_head_after: ff1f08bf4b8d9075ff3761b8106dc01076696337

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "HKDF-Extract(salt = transcript, ikm = secret) then one HKDF-Expand per direction and purpose with a fixed 18-byte info"
    - "key bytes never leave crypto/: DirectionKeys has private fields, no accessor and no Debug"
    - "packet MAC verifies over [0, size - MAC_LEN) before any field but Size is acted on"
    - "Size is the only pre-MAC read, and only to bound the packet"
    - "constant-time tag compare with an accumulator fold and no early exit"

key-files:
  created:
    - crypto/src/session.rs
    - port/src/chan.rs
    - port/tests/aead_transport.rs
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-02.md
  modified:
    - port/Cargo.toml
    - Cargo.lock
    - xtask/src/lintcontract.rs
    - crypto/src/lib.rs
    - port/src/lib.rs

key-decisions:
  - "R4 and the manifest edge land in one commit (Pitfall 7): port may name crypto/ds in every dependency table; aarch64, x86_64 and riscv64 may not"
  - "R4's port exemption is a `continue` on the crate name, so the rule keeps one code path for the arch crates rather than a second per-crate table"
  - "the 18-byte derivation info is label(11) || 0x00 || direction || purpose || generation LE(4), with purposes 0x01 cipher / 0x02 MAC / 0x03 prefix"
  - "seal encrypts the body from block counter 0 and MACs [0, size - MAC_LEN) in one update; open copies the ciphertext out before decrypting because the signature borrows the packet"
  - "open checks my-size bounds and exact length before the MAC, then sequence, then decrypts (T-04-04)"
  - "RED stubs carry the real signatures and wrong bodies (zero keys, always-Ok verify, zero-filled seal), so both tests fail on assertions rather than a compile error (Pitfall 11)"

patterns-established:
  - "Tracer-first vertical slice: derive in crypto, seal/open in port::chan, one host test the existing xtask command already runs"
  - "A pin's RED line records a behavioural assertion failure, never a compile error"

requirements-completed: [WIRE-01, WIRE-03, WIRE-05]

coverage:
  - id: D1
    description: "port links crypto/ and ds/ by path, and lint rule R4 permits that edge for port alone in every dependency table"
    verification:
      - kind: unit
        ref: "xtask/src/lintcontract.rs#lintcontract_accepts_port_depending_on_crypto_and_ds"
        status: pass
      - kind: integration
        ref: "cargo xtask test (runs the lint contract before any leg)"
        status: pass
      - kind: unit
        ref: "xtask/src/lintcontract.rs#lintcontract_rejects_kernel_dependency"
        status: pass
    human_judgment: false
  - id: D2
    description: "crypto::session derives per-direction cipher key, MAC key and nonce prefix from secret, transcript and generation, with no key accessor and no Debug"
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_tracer_one_sealed_packet"
        status: pass
      - kind: integration
        ref: "cargo xtask test"
        status: pass
    human_judgment: false
  - id: D3
    description: "port::chan seals a packet with the 28-byte header at the pinned offsets and opens it back to the same body"
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_tracer_one_sealed_packet"
        status: pass
      - kind: integration
        ref: "cargo xtask test (tracer ok line present)"
        status: pass
    human_judgment: false
  - id: D4
    description: "open_packet refuses a flipped FidHandle byte, a flipped MAC byte, the wrong direction's keys and an unexpected sequence"
    verification:
      - kind: integration
        ref: "port/tests/aead_transport.rs#aead_transport_tracer_rejects_tampering"
        status: pass
    human_judgment: false
  - id: D5
    description: "the tracer pin has RED, GREEN-1, GREEN-2 and ADVANCED lines in the phase evidence"
    verification:
      - kind: other
        ref: "grep -cE '^PIN tracer-sealed-packet (RED|GREEN-1|GREEN-2|ADVANCED)' evidence/PINS-04-02.md == 4"
        status: pass
    human_judgment: false

# Metrics
duration: 4min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 02: Tracer — one sealed packet end to end Summary

**`port` links `crypto`/`ds` under a changed lint rule R4, `crypto::session` derives per-direction keys by HKDF from the handshake transcript, and `port::chan` seals and opens one 28-byte-header packet under `cargo xtask test`.**

## Performance

- **Duration:** 4 min
- **Started:** 2026-11-25T00:18:46Z
- **Completed:** 2026-11-25T00:22:42Z
- **Tasks:** 2
- **Files modified:** 9 (5 modified, 4 created)

## Accomplishments

- `port/Cargo.toml` gains `crypto` and `ds` as path dependencies, and lint-contract rule R4 changed in the same commit to permit that edge for `port` while still rejecting it for `aarch64`, `x86_64` and `riscv64` in every dependency table. `Cargo.lock` gained exactly the two path-dependency lines.
- `crypto::session` holds everything that touches raw key bytes: `derive` (P2 HKDF), `DirectionKeys` (private cipher key, MAC key and 4-byte prefix, no accessor, no `Debug`), `SessionKeys`, `PacketMac` with a constant-time accumulator-fold verify, and `DirectionKeys::keystream` building the `prefix || seq` nonce (D-08).
- `port::chan` pins the 28-byte header and its eight offsets and implements `seal_packet` (header, body encrypted from counter 0, MAC over `[0, size - MAC_LEN)`) and `open_packet` (bounds by `Size`, MAC first, then sequence, then decrypt).
- `port/tests/aead_transport.rs` proves one sealed packet round-trips and that four tampering variants are refused; both tests are picked up by the existing `cargo xtask test` `port` leg with no xtask change.
- The tracer pin is recorded RED → GREEN-1 → GREEN-2 → ADVANCED `3503ea2` in `evidence/PINS-04-02.md`.

## Task Commits

Each task was committed atomically:

1. **Task 1: build edge — `port` depends on `crypto`/`ds`, R4 allows it** - `f78f702` (feat)
2. **Task 2a: RED pin for one sealed packet** - `b6828ad` (test)
3. **Task 2b: session derivation and `port::chan`** - `3503ea2` (feat)
4. **Task 2c: advance the tracer pin** - `ff1f08b` (docs)

**Plan metadata:** the SUMMARY commit follows (docs).

## Files Created/Modified

- `port/Cargo.toml` — `crypto`/`ds` path dependencies with a Phase 4 D-22/D-25 provenance comment.
- `Cargo.lock` — two lines: `"crypto"` and `"ds"` under the `port` package.
- `xtask/src/lintcontract.rs` — R4 now exempts `port` and rejects the three arch crates; header, summary message and fixtures updated (accepting test, arch-crate rejection, dev-dependency and mixed-table cases).
- `crypto/src/lib.rs` — `pub mod session;`.
- `crypto/src/session.rs` — derivation, per-direction keys, packet MAC, keystream.
- `port/src/lib.rs` — `#[forbid(unsafe_code)] pub mod chan;` with a dated provenance comment.
- `port/src/chan.rs` — header constants/offsets, `seal_packet`, `open_packet`.
- `port/tests/aead_transport.rs` — the two tracer tests.
- `evidence/PINS-04-02.md` — the pin ledger (four PIN lines).

## Decisions Made

- R4's exemption is implemented as an early `continue` for `port`, so the arch-crate branch stays one code path and the message names the arch crate and the decision.
- The derivation `info` layout is P2's 18-byte form; purposes and directions are private constants/methods, so the wire contract is pinned by behaviour, not by exported constants.
- `open_packet` copies the ciphertext body out before decrypting because its signature borrows `&[u8]`; that keeps the API read-only on the received buffer with no extra unsafe or in-place trick.
- Fixtures were extended beyond the plan minimum (the named-table `[dependencies.crypto]` form for `port` accepted; an arch crate naming `crypto` in `[dependencies]` and `ds` in `[dev-dependencies]` rejected) so both table shapes are covered.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Replaced a clippy-flagged zero-fill in `seal_packet`**
- **Found during:** Task 2 GREEN verification (`cargo xtask clippy`)
- **Issue:** `Vec::with_capacity(size)` followed by `resize(size, 0u8)` tripped `clippy::slow_vector_initialization` in the new `port/src/chan.rs`.
- **Fix:** Replaced with `alloc::vec![0u8; size]`, which fills once.
- **Files modified:** `port/src/chan.rs`
- **Verification:** `cargo xtask clippy` exits 0 and no warning names `port/src/chan.rs`; `cargo xtask test` re-run twice green after the fix.
- **Committed in:** `3503ea2`

**2. [Rule 2 - Missing Critical] Guarded the 32-bit `Size` field in `seal_packet`**
- **Found during:** Task 2 GREEN implementation
- **Issue:** `Size` is a 32-bit wire field (D-06); a body large enough to push the packet past `u32::MAX` would truncate the field silently and make the packet unopenable.
- **Fix:** `seal_packet` returns `Err("chan: packet exceeds the 32-bit size field")` before writing anything when `size > u32::MAX as usize`. Unreachable at a real negotiated msize, but the field can no longer be silently wrong.
- **Files modified:** `port/src/chan.rs`
- **Verification:** `cargo xtask test`, `check` and `clippy` all exit 0.
- **Committed in:** `3503ea2`

---

**Total deviations:** 2 auto-fixed (1 bug, 1 missing critical)
**Impact on plan:** Both are inside the task's own new file and neither changes the wire bytes or the public surface. No scope creep.

## Issues Encountered

None. The pre-existing warnings (`port/src/devip.rs` unused import, smoltcp `TCPSTAT_*` dead constants) are out of scope and untouched.

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None remaining. The RED commit's stubs (zero keys, always-`Ok` verify, zero-filled seal) were replaced in the GREEN commit; every public function in `crypto::session` and `port::chan` now has its real body.

## Next Phase Readiness

- **Kernel bytes differ from the baseline from commit `f78f702` (commit A) onward.** The dependency edge alone rearranges `.text`; this is accepted (D-25) and nothing is pushed before the owner's end-of-phase byte-change stop. `before.sha256` remains the phase baseline; the after-record is a later plan's and the owner's stop.
- `cargo xtask test`, `cargo xtask check`, `cargo xtask clippy` and `cargo test -p xtask` are all green with the edge, so both kernels and the riscv64 check leg build with `port` linking `crypto/` and `ds/`.
- The dedicated `cargo xtask test-9p` step that names this pin is plan 04-17 (wave 3); at HEAD the pin runs under `nix develop -c cargo xtask test`'s `port` leg.
- Plan 04-09 creates `port/src/fid.rs`; `port/src/chan.rs` currently holds only the header, seal and open — no TLVs, no message bodies, no transport state, no re-key.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- All four created files exist on disk (`crypto/src/session.rs`, `port/src/chan.rs`, `port/tests/aead_transport.rs`, `evidence/PINS-04-02.md`).
- All four commits are ancestors of HEAD (`f78f702`, `b6828ad`, `3503ea2`, `ff1f08b`).
- The pin ledger carries all four `PIN tracer-sealed-packet` lines.
- `cargo xtask test`, `cargo xtask check`, `cargo xtask clippy` and `cargo test -p xtask` exit 0.
