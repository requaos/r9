# Phase 3: Guide Phase 1 - crypto/ and ds/ Foundations - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-24
**Phase:** 3-Guide Phase 1 - crypto/ and ds/ Foundations
**Areas discussed:** MAC and ChaCha20 construction, Unsafe and lint contract, TPHT shape and Funnel, Crate sourcing and kernel reach

---

## MAC and ChaCha20 construction

### W3: MAC primitive

| Option | Description | Selected |
|--------|-------------|----------|
| Keyed BLAKE3 only (Recommended) | One MAC everywhere; matches the RFC and the 32-byte field; Poly1305 not implemented | ✓ |
| BLAKE3 on the wire, Poly1305 for pages | Poly1305 (16-byte tag) for page sealing; adds RFC 8439 Poly1305 pins | |
| Build both, choose in Phase 4 | Implement and pin both; W3 stays open | |

**User's choice:** Keyed BLAKE3 only

### N1: ChaCha20 counter/nonce layout

| Option | Description | Selected |
|--------|-------------|----------|
| RFC 8439: 32-bit counter, 96-bit nonce (Recommended) | IETF layout with published vectors; fits the 12-byte page nonce; wire nonce extended in Phase 4 | ✓ |
| Original ChaCha: 64-bit counter, 64-bit nonce | Fits the 8-byte wire nonce; not the 12-byte page nonce | |
| Both layouts behind one core | Two constructors, two vector sets | |

**User's choice:** RFC 8439 layout

### "HMAC-BLAKE3"

| Option | Description | Selected |
|--------|-------------|----------|
| BLAKE3 keyed mode, truncated (Recommended) | Native keyed hash; capability tag is its first 8 bytes; no HMAC over BLAKE3 | ✓ |
| Real HMAC construction over BLAKE3 | Generic HMAC instantiated with BLAKE3; no published vectors | |
| Leave to Phase 4 | Only full 32-byte keyed BLAKE3 now | |

**User's choice:** BLAKE3 keyed mode, truncated

### Where the corrections are recorded

| Option | Description | Selected |
|--------|-------------|----------|
| PROJECT.md only, sources untouched (Recommended) | As for the four-flag correction in Phase 2 | |
| PROJECT.md and amend the RFC text | Also edit RFC section 3.1 and drop the Poly1305 reading | ✓ |
| PROJECT.md and an errata file | New errata document; sources untouched | |

**User's choice:** PROJECT.md and amend the RFC text
**Notes:** Differs from the Phase 2 handling, where source documents were left as ingested.

---

## Unsafe and lint contract

### W5: unsafe in ds/

| Option | Description | Selected |
|--------|-------------|----------|
| Per-module forbid, tpht is the one floor (Recommended) | Same shape as port/; Roaring and ClientSet forbid | ✓ |
| ds/ fully safe, crate-level forbid | Safe TPHT; bounds check in deref_tiny | |
| Safe first, unsafe only on evidence | Stop and show the site before opening tpht | |

**User's choice:** Per-module forbid, tpht is the one floor
**Notes:** Amended later in the discussion: a second place, one small isolated Roaring SIMD module, is also allowed to hold unsafe (see Crate sourcing).

### N2: lint and audit scope

| Option | Description | Selected |
|--------|-------------|----------|
| Opt in crypto/ and ds/ only (Recommended) | Workspace lints on both; audit gains ds/ section and a crypto/ zero line; x86_64 waits for Phase 5 | ✓ |
| Also opt x86_64 in now | SAFETY comments on every existing x86_64 site | |
| New crates plus a stricter set | Extra denies in the new crates | |

**User's choice:** Opt in crypto/ and ds/ only

### Review rule for tpht unsafe

| Option | Description | Selected |
|--------|-------------|----------|
| No stop; audit ledger and code review (Recommended) | Treated like arch crates; further unsafe modules need approval | ✓ |
| Show me the unsafe sites before commit | One owner stop, as for port/src | |
| Fixed site budget | Sites named in the plan; anything else stops | |

**User's choice:** No stop; audit ledger and code review

### Verifying concurrent unsafe TPHT

| Option | Description | Selected |
|--------|-------------|----------|
| Miri plus threaded stress tests (Recommended) | Miri as its own xtask step | ✓ |
| Add loom model checking too | Exhaustive interleavings; dev dependency and cfg(loom) | |
| Threaded stress tests only | No extra tools | |

**User's choice:** Miri plus threaded stress tests

---

## TPHT shape and Funnel

### W6: home block slots

| Option | Description | Selected |
|--------|-------------|----------|
| 16 slots (Recommended) | Guide 5.2 and compendium layout; "4-slot" recorded as an error | |
| 4 slots | Redesign the header around 4 slots | |
| 16 in the header, measure the spill point | Keep the layout; treat the spill threshold as measured | ✓ |

**User's choice:** 16 in the header, measure the spill point

### How the measurement closes W6

| Option | Description | Selected |
|--------|-------------|----------|
| Owner stop in this phase, then fix one constant (Recommended) | Owner picks from a table of thresholds 4, 8, 12, 16 | |
| Keep it a per-table parameter | Threshold per consumer; 16 as default | |
| I pick from the measurement without a stop | Claude chooses, records evidence, owner reviews at verification | ✓ |

**User's choice:** Claude picks from the measurement without a stop

### Funnel Hashing (DS-07)

| Option | Description | Selected |
|--------|-------------|----------|
| Not built; DS-07 met by absence (Recommended) | Note in ds/ crate docs | ✓ |
| Build a write-once scratchpad table now | ds/src/funnel.rs with corrected design | |
| Not built, plus a guard check | A check against future misuse | |

**User's choice:** Not built; DS-07 met by absence

### IG-D09 lock

| Option | Description | Selected |
|--------|-------------|----------|
| Lock at the end of Phase 3, on green tests (Recommended) | Lock after the TPHT criteria pass | |
| Lock now, in this discussion | Locked today, amended with W5 and the 16-slot layout | ✓ |
| Leave unlocked until Phase 4 uses it | Lock on a real consumer | |

**User's choice:** Lock now, in this discussion
**Notes:** The spill threshold is still to be measured, so it is recorded as outside the lock.

---

## Crate sourcing and kernel reach

### Source of crypto primitives

| Option | Description | Selected |
|--------|-------------|----------|
| Hand-written, zeroize the only dependency (Recommended) | All primitives in safe Rust in crypto/ | |
| Pinned crates.io crates behind crypto/ | sha2, hmac, hkdf, chacha20, blake3, zeroize; portable backends | ✓ |
| Hand-written, crates as test oracles | Upstream crates as dev-dependencies only | |

**User's choice:** Pinned crates.io crates behind crypto/

### Source of Roaring

| Option | Description | Selected |
|--------|-------------|----------|
| Hand-written in ds/src/roaring.rs (Recommended) | Safe Rust, BTreeSet model | |
| Pinned `roaring` crate behind ds/ | Wrap upstream if no_std works | |
| Hand-written, crate as test oracle | Upstream as dev-dependency oracle | |

**User's choice (free text):** "Hand-written in ds/src/roaring.rs, but I think it will need SIMD to fit our usecase so make allowances for small isolated unsafe usage. Use `roaring` crate as a dev-dependency test oracle in addition to the BTreeSet"
**Notes:** This amends the W5 answer: ds/ has two places that may hold unsafe (tpht and one isolated Roaring SIMD module).

### When Roaring SIMD lands

| Option | Description | Selected |
|--------|-------------|----------|
| Scalar now, SIMD seam reserved (Recommended) | Allowance approved now; SIMD code in Phase 5 | ✓ |
| SIMD in Phase 3, host-tested only | Behind a feature the kernel does not enable | |
| SIMD in Phase 3, usable by the kernel | Proof token from with_simd(); touches arch crates | |

**User's choice:** Scalar now, SIMD seam reserved

### Kernel reach

| Option | Description | Selected |
|--------|-------------|----------|
| no_std, cross-compiled, not linked (Recommended) | Checked for both bare-metal targets; no kernel byte change | ✓ |
| Host-only for now | Kernel-target build discovered in Phase 4 | |
| Linked into the kernels now | Kernel byte change and full procedure in Phase 3 | |

**User's choice:** no_std, cross-compiled, not linked

### Where KeyCapability lives

| Option | Description | Selected |
|--------|-------------|----------|
| In crypto/ (Recommended) | Phase 5 segcrypt.rs imports it; no port/src edit | ✓ |
| In port/src/segcrypt.rs as the guide says | port/src edit and kernel byte change now | |
| Key derivation in crypto/, typestate deferred | CRYP-04 partly met; requirement split | |

**User's choice:** In crypto/

---

## Claude's Discretion

- What to do if a pinned crate cannot build for the kernel targets in portable form (the owner declined further questions; Claude recorded: hand-write that primitive against the same vectors and report it).
- How UNSAFE-AUDIT.md treats unsafe inside third-party crates (recorded: one line per dependency, no upstream site enumeration).
- The spill-threshold selection rule and evidence format.
- Module layout, crate versions and features, xtask wiring, the Miri step name, order of work.

## Deferred Ideas

- Roaring SIMD kernels: Phase 5.
- Funnel Hashing: with its first scratchpad consumer.
- x86_64 lint and audit opt-in: Phase 5.
- Wire sequence nonce to 12-byte ChaCha20 nonce rule: Phase 4.
- Linking crypto/ and ds/ into the kernels: Phases 4 and 5.
