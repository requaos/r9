---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 09
subsystem: docs
tags: [decision-records, project-md, rfc-amendment, state-md, requirements, roadmap, w3, w5, w6, n1, n2, ig-d09, owner-review]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "Plan 03-04: keyed BLAKE3 as the only MAC, `K_file`, the labelled expand and the `KeyCapability` typestate with its type-level guarantee"
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "Plan 03-05: Roaring and `ClientSet`, the hand-written data structures and the D-20 `setops` seam"
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "Plan 03-08: `SPILL_THRESHOLD = 16` at `HIT_RATE_OCCUPANCY = 1.00`, the evidence file with both index variants, and the measured over-90% home-hit pin"
provides:
  - "PROJECT.md: W3, N1, W5, W6, N2 and the IG-D09 lock recorded as dated Key Decisions rows; nine planning-level rows marked for owner review at Phase 3 verification; the safety paragraph and Constraints line state the four-crate lint contract"
  - "`.planning/rfc-9p2000-aead.md` section 3.1: the RFC 8439 32-bit counter and 96-bit nonce, Keyed BLAKE3 as the only MAC, and the dated amendment note"
  - "STATE.md: the open-question table at seven rows, the corrected landing-site blocker, and the two deferred items (aarch64 clippy, TPHT neighbourhood displacement)"
  - "REQUIREMENTS.md: CRYP-02, CRYP-03, CRYP-04, DS-01, DS-07 and LINT-01 name the settled answer"
  - "ROADMAP.md Phase 3 open-questions line carries the `Settled 2026-11-24` prefix after its label"
affects: [03-10, phase-4, phase-5]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 9775
  tasks: 2
  commits: 2
  plan_head_before: fceff9edf634b4e0243372bd3e9bc3a4ca3655c8
  plan_head_after: f0fc698

tech-stack:
  added: []
  patterns:
    - "Two outcome wordings keep the record honest: owner decisions are `Decided/Locked amended 2026-11-24 (owner)`, planning-level choices are `Recorded 2026-11-24 (planning; for owner review at Phase 3 verification)`"
    - "Superseded wording is named in the rationale column of every corrected decision (guide 6.3 and compendium 1.2/3.2 for W3; RFC 3.1 for N1; guide 11 for `ds/`; guide 5.3 for W6), and the RFC text itself is amended rather than only PROJECT.md"
    - "Scalar W3/W5/W6 rewrites: the IG-D09 row is rewritten in place because it was never locked; rows appended below it are new dated rows"
    - "The guide and compendium stay byte-identical to the baseline commit; every correction lives in PROJECT.md and the RFC"
    - "Values read from the code and evidence at execution time: `SPILL_THRESHOLD = 16` at occupancy 1.00, `=1.9.2` for zeroize, block index `u32::MAX - 1` and 274,877,906,880 bytes for ChaCha20"

key-files:
  created: []
  modified:
    - .planning/PROJECT.md
    - .planning/rfc-9p2000-aead.md
    - .planning/STATE.md
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md

key-decisions:
  - "W3 row: Keyed BLAKE3 is the only MAC in r9 (wire packets, page sealing, capability tags), Poly1305 is not implemented; supersedes '/ Poly1305' in guide 6.3 and compendium 1.2 and 3.2; outcome `Decided 2026-11-24 (owner)`"
  - "N1 row: ChaCha20 uses the RFC 8439 layout, a 32-bit block counter and a 96-bit nonce, pinned to RFC 8439 2.3.2 and 2.4.2; supersedes the RFC 3.1 64-bit-counter wording; the RFC text was amended and the wire-nonce extension stays Phase 4's; outcome `Decided 2026-11-24 (owner)`"
  - "IG-D09 row rewritten in place: succinct TPHT is the primary concurrent hashing, Funnel Hashing is scratchpad only and not built, `ds/src/tpht/` is an unsafe-holding module, the home header has 16 slots; the spill threshold and the home-index bits are not part of the lock; outcome `Locked amended 2026-11-24 (owner)`"
  - "W5/N2 row records the two-crate lint contract: crate-level forbid in `crypto/`, per-module forbids in `ds/` with `ds/src/tpht/` and the reserved Roaring SIMD module as the only unsafe holders, both lint-opted-in, every `tpht` site SAFETY-commented and audited, no owner diff stop"
  - "W6 row records 16 home slots and the measured `SPILL_THRESHOLD = 16` at occupancy 1.00, citing `evidence/SPILL-THRESHOLD.md`; the threshold measurement is for owner review at verification"
  - "RFC section 3.1 item 1 now states the RFC 8439 layout; item 2 states Keyed BLAKE3 is the only MAC and [RFC8439] is cited for ChaCha20 only; the dated note records the old 64-bit-counter wording, the `HMAC-BLAKE3` reading (D-07) and that the 8-to-12-byte nonce rule is Phase 4's"
  - "Nine planning-level rows (compendium 4.1 index correction, type-level `ReadOnly`, MurmurHash3 `f(R)`, `K_file` info encoding, parallel chain-head array, Boehm seqlock, build/gate readings, ChaCha20 per-key limit, `zeroize` choice) carry the `for owner review at Phase 3 verification` outcome and cite the RESEARCH finding"
  - "STATE.md keeps seven open warnings (W1, W2, W4, W7-W10), records that `crypto/` and `ds/` now exist and are host-tested, and defers the pre-existing aarch64 clippy failure plus the TPHT neighbourhood-displacement gap"
  - "The guide and the compendium are not edited; `git diff --quiet` against baseline `4beb5c8` passes for both"

requirements-completed: [CRYP-02, CRYP-03, CRYP-04, DS-01, DS-07, LINT-01]

coverage:
  - id: D1
    description: "PROJECT.md records W3, N1, W5, W6, N2 and the IG-D09 lock as dated Key Decisions rows, with the Open Questions table reduced to W1, W2, W4, W7-W10"
    requirement: DS-01
    verification:
      - kind: other
        ref: "grep -c 'Locked amended 2026-11-24 (owner)' = 1; grep -cE '^\\| W(3|5|6) \\|' = 0; grep -c 'Decided 2026-11-24 (owner' = 8; grep -c 'Recorded 2026-11-24 (planning' = 9; nine-subject grep MISSING = 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "RFC section 3.1 states the 32-bit block counter and 96-bit nonce, Keyed BLAKE3 as the only MAC, and carries the dated amendment note; [RFC8439] is marked 'cited for ChaCha20 only'"
    requirement: CRYP-02
    verification:
      - kind: other
        ref: "grep -cE '^1\\. \\*\\*Confidentiality:\\*\\*.*32-bit block counter' = 1; grep -c 'Amended 2026-11-24' = 1; grep -c 'Keyed BLAKE3 is the only MAC' = 1; `git show f0fc698 -- .planning/rfc-9p2000-aead.md` touches section 3.1 and the RFC8439 reference line only"
        status: pass
    human_judgment: false
  - id: D3
    description: "STATE.md holds seven open-question rows, the corrected landing-site blocker and the two new Deferred Items rows (aarch64 clippy; TPHT neighbourhood displacement)"
    requirement: LINT-01
    verification:
      - kind: other
        ref: "grep -cE '^\\| W(3|5|6) \\|' .planning/STATE.md = 0; grep -c 'clippy --arch aarch64' = 1; Deferred Items table has the Lint and Design rows"
        status: pass
    human_judgment: false
  - id: D4
    description: "REQUIREMENTS.md wording for CRYP-02, CRYP-03, CRYP-04, DS-01, DS-07 and LINT-01 names the settled answer, with every checkbox unchanged by this plan"
    requirement: LINT-01
    verification:
      - kind: other
        ref: "`git diff fceff9e..f0fc698 -- .planning/REQUIREMENTS.md` shows the six wording edits and no checkbox state change; line 6 lists W1, W2, W4 and W7-W10 as open and W3, W5, W6 as settled"
        status: pass
    human_judgment: false
  - id: D5
    description: "The Phase 3 ROADMAP open-questions line keeps its `**Open questions**:` label and gains the `Settled 2026-11-24 (see PROJECT.md Key Decisions):` prefix; no other phase's line changed"
    requirement: CRYP-02
    verification:
      - kind: other
        ref: "grep -c '^\\*\\*Open questions\\*\\*: Settled 2026-11-24 (see PROJECT.md Key Decisions): W3' .planning/ROADMAP.md = 1; `git diff --stat fceff9e..f0fc698 -- .planning/ROADMAP.md` = 1 insertion, 1 deletion"
        status: pass
    human_judgment: false
  - id: D6
    description: "The guide and the compendium are byte-identical to baseline `4beb5c8`; no source, manifest, flake or UNSAFE-AUDIT edit was made"
    requirement: LINT-01
    verification:
      - kind: other
        ref: "B=$(sed -n 's/^base //p' evidence/kernel-bytes/before.sha256); git diff --quiet \"$B\" HEAD -- .planning/r9-implementation-guide-amended-v3.md .planning/r9-architecture-and-code-v4.md exits 0"
        status: pass
    human_judgment: false
  - id: D7
    description: "The nine planning-level choices are distinguishable from owner decisions and reachable in one table, for the owner's review at Phase 3 verification"
    requirement: DS-01
    verification: []
    human_judgment: true
    rationale: "Whether the owner accepts the nine recorded choices (compendium 4.1 correction, type-level ReadOnly, f(R), K_file encoding, chain-head array, seqlock ordering, build/gate readings, ChaCha20 limit, zeroize pin) is the verification's human judgement; the mechanical half is the nine `Recorded 2026-11-24 (planning` outcomes"

# Metrics
duration: 1min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 09: Decision records — PROJECT.md, the RFC amendment, STATE, REQUIREMENTS and ROADMAP

**Phase 3's settled questions are written where later phases read them: PROJECT.md holds eight owner-decision rows (W3, N1, D-07, W5/N2, W6, D-18 to D-20, D-21/D-22, D-13) plus nine planning-level rows for owner review, the IG-D09 row is locked amended 2026-11-24, RFC section 3.1 states the RFC 8439 counter and nonce and Keyed BLAKE3 as the only MAC with a dated note, and STATE, REQUIREMENTS and ROADMAP now agree with all of it while the guide and compendium stay byte-identical.**

## Performance

- **Duration:** 1 min
- **Started:** 2026-11-24T16:53:25Z
- **Completed:** 2026-11-24T16:54:14Z
- **Tasks:** 2
- **Files modified:** 5 (all planning documents; no source, manifest, flake or audit file touched)

## Accomplishments

- **PROJECT.md Open Questions.** W3, W5 and W6 left the table, which now holds W1, W2, W4, W7, W8, W9 and W10 (seven rows); the sentence above it says so and records that the three were settled in Phase 3 on 2026-11-24.
- **PROJECT.md safety boundary.** Both the safety paragraph and the Constraints `Safety` line state the Phase 3 contract: `undocumented_unsafe_blocks = deny` is opted into by `aarch64`, `port`, `crypto` and `ds`; `crypto/` is crate-level `forbid(unsafe_code)` over r9's own code and not the pinned crates it wraps; `ds/` uses per-module forbids with exactly two unsafe-holding places, `ds/src/tpht/` and one reserved Roaring SIMD module with no code until Phase 5; a third needs owner approval. The "open question W5" clause is gone.
- **PROJECT.md Key Decisions.** The IG-D09 row is rewritten in place: succinct TPHT as the primary concurrent hashing, Funnel Hashing scratchpad only and not built, `ds/src/tpht/` as the unsafe-holding module, 16 home slots, and the spill threshold and home-index bits explicitly outside the lock; outcome `Locked amended 2026-11-24 (owner)`. The paragraph above the table now counts eight locked, three locked amended and two deferred (IG-D11 to Phase 4, IG-D13 to Phase 6).
- **Eight owner rows appended**, each naming what it supersedes: W3 (Keyed BLAKE3 only; the guide 6.3 and compendium 1.2/3.2 wording corrected; RFC amended), N1 (RFC 8439 32-bit counter and 96-bit nonce; the RFC's 64-bit wording corrected), D-07 (`HMAC-BLAKE3` means keyed BLAKE3 truncated), W5/N2 (the two-crate lint contract, superseding guide section 11 for `ds/`), W6 (16 slots; `SPILL_THRESHOLD = 16` at occupancy 1.00; evidence path), D-18 to D-20 (six pins with versions; hand-written Roaring, TPHT and `ClientSet`; `roaring` as oracle; SIMD is Phase 5), D-21/D-22 (no kernel dependency; `KeyCapability` lives in `crypto/` and `port/src/segcrypt.rs` moves), and D-13 (Miri plus stress, no `loom`).
- **Nine planning-level rows appended**, each citing its RESEARCH finding and carrying `Recorded 2026-11-24 (planning; for owner review at Phase 3 verification)`: the compendium 4.1 mixed-half index correction with both measured variants, the type-level `ReadOnly` guarantee, MurmurHash3 `f(R)`, the `K_file` 12-byte little-endian `info`, the parallel chain-head array and DS-04 fill definition, the Boehm seqlock, the build/gate readings (r9 target specs, `R9_MIRI_TOOLCHAIN`, the clippy command, the DS-06 proof), the ChaCha20 per-key limit (`u32::MAX - 1`, 274,877,906,880 bytes), and the `zeroize =1.9.2` pin with `=1.9.1` as the older alternative.
- **RFC section 3.1 amended.** Item 1 states the RFC 8439 layout (32-bit block counter, 96-bit nonce); item 2 adds that Keyed BLAKE3 is the only MAC of the protocol, Poly1305 is not used and [RFC8439] is cited for ChaCha20 only; the dated note names the old 64-bit wording, the `HMAC-BLAKE3` reading and the Phase 4 nonce-extension rule; the section 13 reference appends "(cited for ChaCha20 only)". Nothing else in the RFC changed.
- **STATE.md.** The open-question table is at seven rows and its heading says seven; the landing-site blocker says `crypto/` and `ds/` now exist and are host-tested while `kern/`, `sys/` and `venti/` are still absent; Deferred Items gains the pre-existing aarch64 clippy failure (7 undocumented unsafe blocks, with the seven file:line sites and the possibly stale audit count of 129) and the TPHT neighbourhood-displacement gap.
- **REQUIREMENTS.md.** CRYP-02 names the settled layout (D-06), CRYP-03 names Keyed BLAKE3 as the only MAC, CRYP-04 carries the type-level qualification and the `crypto/` landing site (D-22), DS-01 names 16 header slots and `SPILL_THRESHOLD`, DS-07 records that Funnel Hashing is not built (D-16), LINT-01 states the full lint contract; line 6 no longer says every warning is open. No checkbox state changed.
- **ROADMAP.md.** The Phase 3 line keeps `**Open questions**:` first and inserts `Settled 2026-11-24 (see PROJECT.md Key Decisions): ` directly after it; one line changed and no other phase's line touched.
- **Transparency.** The guide (`.planning/r9-implementation-guide-amended-v3.md`) and the compendium (`.planning/r9-architecture-and-code-v4.md`) are byte-identical to baseline `4beb5c8`; no source, manifest, `flake.nix` or `UNSAFE-AUDIT.md` file was touched.

## Task Commits

1. **Task 1: PROJECT.md — Open Questions, notes N1 and N2, Key Decisions rows, the safety paragraphs** — `9703a0a` (docs)
2. **Task 2: the RFC amendment, STATE.md, REQUIREMENTS.md and the ROADMAP open-questions line** — `f0fc698` (docs)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Files Created/Modified

- `.planning/PROJECT.md` — Open Questions reduced to seven rows; N1 and N2 appendix paragraphs mark them settled; the safety paragraph and Constraints `Safety` line state the four-crate lint contract; the Key Decisions preamble counts eight locked / three locked amended / two deferred; the IG-D09 row is rewritten and 17 dated rows are appended; the footer date line updated
- `.planning/rfc-9p2000-aead.md` — section 3.1 items 1 and 2 plus the dated amendment note; the [RFC8439] reference gains "(cited for ChaCha20 only)"
- `.planning/STATE.md` — the open-question heading and table; the landing-site blocker; two Deferred Items rows
- `.planning/REQUIREMENTS.md` — line 6 and the six requirement lines named above
- `.planning/ROADMAP.md` — the Phase 3 open-questions line prefix only

## Decisions Made

- **Values were read at execution time, not from the plan.** `SPILL_THRESHOLD = 16` and `HIT_RATE_OCCUPANCY = 1.00` come from `ds/src/tpht/mod.rs` and the evidence `CHOSEN` lines; the ChaCha20 figures (last block index 4294967294, 274,877,906,880 bytes) from `crypto/src/page.rs` and the `BOUNDARY chacha20` ledger line; the `zeroize` version `=1.9.2` from `crypto/Cargo.toml`. They agree with the plan's expectations.
- **The nine planning-level rows carry the RESEARCH finding in the rationale column** (C1, C4, C6/A5, F7, Pitfalls 9, Open Questions 1-7), so the owner can trace each to its evidence without opening the plan.
- **The IG-D09 row was rewritten, not superseded by a second row.** IG-D09 was never locked, so the rewrite is the lock itself (D-17); every other change to a locked decision would have been a new dated row.
- **The stale summary row `Eleven ingest warnings: ten carried as open questions, W11 decided | Ten pending` was left untouched.** The plan's acceptance criterion limits the table change to the IG-D09 rewrite plus appended rows; the Open Questions section and the new rows state the settled count, so the record is unambiguous. Flagged here for the owner's reading.
- **The PROJECT.md footer date line was updated** to name the Phase 3 decision records; this is metadata outside the Key Decisions table.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. All six Task 1 verify gates and all seven Task 2 verify gates pass at the final state; the guide and compendium baseline check (`git diff --quiet 4beb5c8 HEAD -- <both source documents>`) exits 0; documents only, so no build, test, clippy or Miri run was required by this plan.

## Threat Flags

None beyond the plan's `<threat_model>`. T-03-33 (a planning-level choice recorded as an owner decision) is mitigated by the two outcome wordings and the verify counts (8 `Decided`, 9 `Recorded`); T-03-34 (a later phase implementing superseded wording) by the RFC text amendment and by every corrected row naming what it supersedes; T-03-35 (a recorded value differing from the code) by reading `SPILL_THRESHOLD`, the ChaCha20 figures and the `zeroize` version from the code and evidence at execution time. T-03-SC does not apply (no package was added).

## Known Stubs

None. Every value recorded is read from the code or the evidence; no placeholder text, TODO or unwired subject remains in the five files.

## For owner review

- **The nine planning-level rows** in PROJECT.md's Key Decisions table, each marked `Recorded 2026-11-24 (planning; for owner review at Phase 3 verification)`: compendium 4.1 mixed-half index correction; `ReadOnly` as a type-level guarantee; MurmurHash3 `f(R)`; the `K_file` little-endian `info`; the parallel chain-head array and DS-04 fill definition; the Boehm seqlock; the build/gate readings; the ChaCha20 per-key limit; the `zeroize =1.9.2` pin with `=1.9.1` as the older alternative (`03-OWNER-STOP.md`, written by plan 03-10, carries the last choice).
- **The W6 threshold.** `SPILL_THRESHOLD = 16` at `HIT_RATE_OCCUPANCY = 1.00`; the weakest of the fifteen runs is sequential 236,035 / 262,144 = 0.900402, only 0.000402 above the 0.90 bar. The evidence is `evidence/SPILL-THRESHOLD.md` (rule at `b479d28`, measured tables at `4150faf`).
- **The ChaCha20 limit per key and nonce.** Last block index `u32::MAX - 1`, so one key and nonce cover at most 2^32 - 1 blocks, 274,877,906,880 bytes from counter 0 — one 64-byte block fewer than RFC 8439. Phase 4 must size what it encrypts under one nonce against this figure.
- **The stale `Ten pending` summary row** noted under Decisions Made; left as-is to honour the plan's acceptance criterion.

## Next Phase Readiness

- Plan 03-10 can write the owner checklist against the nine planning-level rows and the two open choices (`zeroize` and the ChaCha20 limit); its `UNSAFE-AUDIT.md` work is unaffected.
- Phase 4 builds the wire format on W3 (Keyed BLAKE3 only, 32-byte MAC field), N1 (32-bit counter, 96-bit nonce) and the Phase 4 nonce-extension rule; W1, W2 and W4 remain open.
- Phase 5 imports `KeyCapability` from `crypto/` at `port/src/segcrypt.rs` (D-22) and adds the Roaring SIMD module behind `setops`; the `x86_64` lint extension stays note N2's Phase 5 question.
- The aarch64 clippy failure and the TPHT neighbourhood-displacement gap are recorded in STATE.md Deferred Items for whichever phase takes them.

---

*Phase: 03-guide-phase-1-crypto-and-ds-foundations*
*Completed: 2026-11-24*

## Self-Check: PASSED

All five plan files exist on disk (`.planning/PROJECT.md`, `.planning/rfc-9p2000-aead.md`, `.planning/STATE.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`) and both task commits (`9703a0a`, `f0fc698`) are ancestors of HEAD. Plan-level verification at the final state: Task 1's six gates print `Locked amended 2026-11-24 (owner)` = 1, open W3/W5/W6 rows = 0, `Decided 2026-11-24 (owner` = 8, `Recorded 2026-11-24 (planning` = 9, nine-subject MISSING = 0; Task 2's gates print the RFC item-1 counter line = 1, `Amended 2026-11-24` = 1, `Keyed BLAKE3 is the only MAC` = 1, STATE W3/W5/W6 rows = 0, `clippy --arch aarch64` = 1, the ROADMAP settled prefix = 1; and `git diff --quiet 4beb5c8 HEAD -- .planning/r9-implementation-guide-amended-v3.md .planning/r9-architecture-and-code-v4.md` exits 0, so the guide and compendium are byte-identical to the baseline. No build, test, clippy or Miri run is required by this documents-only plan.
