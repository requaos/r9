---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 02
subsystem: infra
tags: [rfork, abi, threat-model, xtask, nyquist-validation, v3.0-rescope, debt, correction]

requires:
  - phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools (plan 05-01)
    provides: green cargo xtask test harness carrying the xtask unit legs and test-9p
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (v3.0, archived)
    provides: the completed verification/UAT evidence DEBT-02 reconciles, and the owner-stop-2 boot-iso evidence DEBT-03 corrects
provides:
  - port::rfork with RFNOLEGACY = 1 << 18 and the three retired crypto flags removed (CORR-04, D-02)
  - regenerated sys/include/rfork.h pinned by cargo xtask rfork-header --check
  - docs/threat-model.md rescoped to the v3.0 hardware MMU boundary with bit-18 RFNOLEGACY
  - xtask/tests/threat_model.rs validating against docs/plan_r9_architecture_code_compendium_v3_0.md
  - archived 04-VALIDATION.md reconciled to status: validated / nyquist_compliant: true / wave_0_complete: true (DEBT-02, D-34)
  - dated DEBT-03 correction notes retracting the moved-linked-kernel-bytes claim in Phase 4 evidence
affects: [phase-05 later plans (CORR-03/05, SMP, MEM), phase-06 RFNOLEGACY enforcement, phase-07 notes]

actuals:
  tokens: 20601
  tasks: 3
  commits: 3
  plan_head_before: bf9e7d0eed4c0dc657cec06ba5bf2fad2bf9fd20
  plan_head_after: f43059c41ba459f87e81d199ae0db87bc77d1528

tech-stack:
  added: []
  patterns:
    - "Single-source ABI: port/src/rfork.rs is the one definition; sys/include/rfork.h is generated and CI-checked; retiring a flag edits both arch mask tests in the same commit"
    - "Threat-model contract test pins exact wording (MMU boundary quote, bit-18 RFNOLEGACY) and rejects stale crypto-memory protections in section 5"

key-files:
  created: []
  modified:
    - port/src/rfork.rs
    - sys/include/rfork.h
    - xtask/src/rforkh.rs
    - port/tests/rfork_test.rs
    - x86_64/src/syscall.rs
    - aarch64/src/syscall.rs
    - docs/threat-model.md
    - xtask/tests/threat_model.rs
    - .planning/milestones/v3.0-phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-VALIDATION.md
    - .planning/milestones/v3.0-phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/KERNEL-BYTES-GAPS.md
    - .planning/milestones/v3.0-phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-25-SUMMARY.md

key-decisions:
  - "Retirement doc trail lives in code comments: port/src/rfork.rs names the 2026-11-25 rescope (CORR-04, D-02) and states bits 19-21 are free, so the ABI file carries its own provenance"
  - "Threat model keeps its eight-section shape and citation discipline; the AC tag is redefined against docs/plan_r9_architecture_code_compendium_v3_0.md and the memory boundary quotes the compendium's Retained bullet verbatim so doc and compendium cannot drift apart"
  - "04-VALIDATION.md reconciliation fills the map from 04-VERIFICATION.md (109/109) and 04-UAT.md (112/112) rather than re-running the phase, per D-34"

patterns-established:
  - "A dropped mechanism is named in the threat model only as removed-from-scope; section 5 rows may never rest on one (enforced by threat_model_has_no_stale_crypto_memory_protections)"

requirements-completed: [CORR-04, DEBT-02, DEBT-03, DEBT-05]

coverage:
  - id: D1
    description: "port/src/rfork.rs defines RFNOLEGACY = 1 << 18 with RFCKEY/RFROBASE/RFRANDMEM removed; sys/include/rfork.h regenerated and pinned by rfork-header --check"
    requirement: CORR-04
    verification:
      - kind: unit
        ref: "nix develop -c cargo xtask rfork-header --check (matches); cargo test -p port rfork (3 passed); cargo test -p xtask rforkh (5 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "docs/threat-model.md states the hardware MMU isolation boundary, removes software pages-at-rest/stale-frame/shadow-frame/ymm/PKEY/sibling-key items from scope, and states bits 18-20 retired with RFNOLEGACY at bit 18 and bits 19-21 free; contract test validates against the v3.0 compendium"
    requirement: DEBT-05
    verification:
      - kind: unit
        ref: "nix develop -c cargo test -p xtask --test threat_model (10 passed, incl. boundary quote, exact bit-18 wording, stale-protection rejection)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Archived 04-VALIDATION.md reconciled to status: validated / nyquist_compliant: true / wave_0_complete: true with a zero-TBD verification map filled from the completed Phase 4 evidence"
    requirement: DEBT-02
    verification:
      - kind: other
        ref: "grep -cE '^(status: validated|nyquist_compliant: true|wave_0_complete: true)$' == 3; zero TBD cells"
        status: pass
    human_judgment: false
  - id: D4
    description: "Dated DEBT-03/INT-02 correction notes in KERNEL-BYTES-GAPS.md and 04-25-SUMMARY.md retract the moved-linked-kernel-bytes claim, citing sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I= and Phase 4 D-16/D-23"
    requirement: DEBT-03
    verification:
      - kind: other
        ref: "grep -c 'DEBT-03' >= 1 in both files; SHA and D-16/D-23 present in both"
        status: pass
    human_judgment: false

duration: 11min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 02: Course Correction — Retired rfork Bits, v3.0 Threat Model, Phase 4 Record Reconciliation Summary

**`RFCKEY`/`RFROBASE`/`RFRANDMEM` removed from `port::rfork` with `RFNOLEGACY` moved to `1 << 18` and the generated C header re-pinned (CORR-04); `docs/threat-model.md` rescoped to the v3.0 hardware-MMU-only memory boundary with a contract test that pins the exact wording against the v3.0 compendium (DEBT-05); and the archived Phase 4 validation record reconciled to `validated` with dated DEBT-03 retractions of the unsupported kernel-bytes claim (DEBT-02/03).**

## Performance

- **Duration:** 11 min
- **Started:** 2026-11-26T04:29:08Z
- **Completed:** 2026-11-26T04:40:13Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments

- **CORR-04:** `port/src/rfork.rs` now defines thirteen flags: the twelve Harvey values plus `RFNOLEGACY = 1 << 18`, with the three cryptographic bits retired and bits 19-21 documented as free (D-02). `sys/include/rfork.h` regenerated via `cargo xtask rfork-header` (`RFNOLEGACY = (1<<18)`); `xtask/src/rforkh.rs` pins the new enumerator line, asserts the retired names never appear in the header, and moves the drift-detection mutation to `(1<<18)`. Verified: `rfork-header --check` matches, `cargo test -p port rfork` 3/3, `cargo test -p xtask rforkh` 5/5, and the arch mask tests (`rfork_reserved_bits_are_unsupported`, aarch64 host leg) pass with `RFORK_SUPPORTED` unchanged at `0x3c7f` — runtime rfork behaviour is unchanged.
- **DEBT-05:** `docs/threat-model.md` rewritten to the v3.0 rescope: hardware MMU isolation (`CR3`/`TTBR0`/`satp` + PTE `RO`/`RW`, per-process page tables, read-only user text, kernel/user separation) stated as the exact memory protection boundary, quoting the v3.0 compendium's Retained bullet verbatim; software protection of pages at rest, stale-frame/shadow-frame encryption, `ymm0`-`ymm5` sealing, `PKEY`/`WRPKRU` hooks and sibling-key isolation explicitly removed from scope with the 2026-11-25 date and the SEV-SNP/TDX remedy retained for physical DRAM threats; the retired-bits statement (bits 18-20 retired with software cryptographic memory, `RFNOLEGACY` is bit 18, bits 19-21 free) recorded; the live `9P2000.aead` wire, `FidTable` handle and legacy-gateway rows retained. `xtask/tests/threat_model.rs` now validates against `docs/plan_r9_architecture_code_compendium_v3_0.md` (not the superseded v4 compendium), pins the exact boundary quote and bit-18 wording, and rejects any section-5 protection row resting on a dropped mechanism — 10/10 tests pass.
- **DEBT-02:** archived `04-VALIDATION.md` reconciled from `status: draft` / `nyquist_compliant: false` / `wave_0_complete: false` to `validated` / `true` / `true` with `validated_at: "2026-11-25"`; the per-task verification map's TBD cells filled from `04-VERIFICATION.md` (109/109 must-haves, 42/42 gap-closure truths) and `04-UAT.md` (112/112), and the sign-off checklist completed — per D-34, no archived requirement rows touched.
- **DEBT-03:** dated correction notes (`2026-11-25 — DEBT-03 / v3.0 audit INT-02`) added to `KERNEL-BYTES-GAPS.md` (both affected passages) and `04-25-SUMMARY.md`, retracting the claim that the Phase 4 fixes moved the linked kernel bytes: the unchanged `boot-iso` hash `sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=` shows only unstripped DWARF/line metadata moved on the Mac ELF while the stripped linked kernel bytes stayed identical; the fixes are verified by the host `test-9p` suite because a default boot speaks plain `9P2000` (Phase 4 D-16/D-23).

## Task Commits

Each task was committed atomically:

1. **Task 1: Retire crypto rfork flags, move RFNOLEGACY to bit 18, and regenerate sys/include/rfork.h (CORR-04)** - `3a57171` (feat)
2. **Task 2: Align docs/threat-model.md and xtask/tests/threat_model.rs with the v3.0 rescope (DEBT-05)** - `6eac717` (docs)
3. **Task 3: Reconcile archived Phase 4 validation record and correct boot-iso kernel-bytes evidence (DEBT-02, DEBT-03)** - `f43059c` (docs)

**Plan metadata:** this SUMMARY commit (docs: complete plan)

## Files Created/Modified

- `port/src/rfork.rs` — three retired crypto flags removed, `RFNOLEGACY = 1 << 18`, module doc records the rescope and free bits 19-21
- `sys/include/rfork.h` — regenerated from `port::rfork`
- `xtask/src/rforkh.rs` — header assertions pin `(1<<18)` and reject retired names; drift strings updated; banner singular
- `port/tests/rfork_test.rs` — pins updated for the 13-flag table (`ALL.len() == 13`, disjoint mask `0x47c7f`)
- `x86_64/src/syscall.rs`, `aarch64/src/syscall.rs` — `rfork_mask_tests` import and assert only the two remaining non-implemented flags
- `docs/threat-model.md` — full v3.0 rescope rewrite (boundary, assets, attackers, trust boundaries, protections, out-of-scope, open points, sources)
- `xtask/tests/threat_model.rs` — validates against the v3.0 compendium; four new pins (v3.0 AC definition, MMU boundary, bit-18 wording, no stale crypto-memory protections)
- `04-VALIDATION.md` — reconciled frontmatter, filled verification map, completed sign-off
- `KERNEL-BYTES-GAPS.md`, `04-25-SUMMARY.md` — dated DEBT-03 correction notes

## Decisions Made

- **Provenance in the ABI file:** the retirement is documented in `port/src/rfork.rs`'s own module doc (CORR-04, D-02, bits 19-21 free), so the single source of truth carries its history rather than relying on planning documents alone.
- **Contract-by-quote kept:** the new threat-model test keeps the old verbatim-quote discipline, now against the v3.0 compendium's Retained bullet, plus exact-wording pins for the bit-18 statement and a rejection list for stale framings — the doc and the compendium cannot drift apart unnoticed.
- **Reconciliation from completed evidence:** DEBT-02 fills the validation map from the already-run verification (109/109) and UAT (112/112) records rather than re-executing the phase, as D-34 intends.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated arch mask tests and port rfork pins that referenced the removed constants**
- **Found during:** Task 1 (CORR-04)
- **Issue:** The plan's `<files>` named only `port/src/rfork.rs`, `sys/include/rfork.h` and `xtask/src/rforkh.rs`, but removing `RFCKEY`/`RFROBASE`/`RFRANDMEM` breaks compilation of `port/tests/rfork_test.rs` and the `rfork_mask_tests` modules in `x86_64/src/syscall.rs` and `aarch64/src/syscall.rs`, which import the retired constants. The plan's action text anticipates this ("update any unit tests ... referencing the old bit positions or removed flags") but the tests live outside `port/src/rfork.rs`.
- **Fix:** Updated the three test files in the same commit: `rfork_reserved_values_are_pinned` now pins only `RFNOLEGACY = 1 << 18`; `ALL.len() == 13` and disjoint mask `0x47c7f`; both arch mask tests iterate `RFNOMNT` + `RFNOLEGACY` and keep `RFORK_SUPPORTED == 0x3c7f`.
- **Files modified:** `port/tests/rfork_test.rs`, `x86_64/src/syscall.rs`, `aarch64/src/syscall.rs`
- **Verification:** `cargo test -p port rfork` 3/3; `cargo test -p xtask rforkh` 5/5; aarch64 host leg `rfork_reserved_bits_are_unsupported` ok; x86_64 test module is byte-identical to the aarch64 one (its host leg runs on x86_64 CI runners)
- **Committed in:** `3a57171` (part of task commit)

---

**Total deviations:** 1 auto-fixed (Rule 3 - blocking)
**Impact on plan:** Required for compilation after the flag removal; no scope creep — `RFORK_SUPPORTED` masks and runtime behaviour unchanged.

## Issues Encountered

- Plain `cargo test -p x86_64` / `-p aarch64` cannot run on this host (the `no_std` arch crates need `--bins --target <host-triple>`, and the x86_64-apple-darwin std is absent from the nix toolchain — pre-existing environment limitation). Verified the aarch64 mask test via the host-triple leg and confirmed the x86_64 module is byte-identical. No plan impact.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None.

## Next Phase Readiness

- The rfork ABI is at its v3.0 state: `RFNOLEGACY = 1 << 18` in Rust and C with bits 19-21 free, ready for Phase 6 enforcement and Phase 7 parity work (D-02).
- The threat model is a trustworthy contract for the remaining Phase 5 memory work (MEM): it states the hardware MMU boundary the pools and evictor rely on, and its test now guards against reintroducing dropped mechanisms.
- The v3.0 audit follow-ups DEBT-02 and DEBT-03 are closed; the remaining audit item for this phase is DEBT-04 (TPHT bank sizing) ahead of the page cache (D-18).
- `Cargo.lock` unchanged; zero external packages installed (T-05-SC package-legitimacy gate N/A). No kernel byte change from this plan (comment/test/docs edits only in kernel crates), so no boot-iso hash procedure applies; green CI on the merge commit is the check (D-05, post-CORR-02).

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- All 12 named files (11 modified + this SUMMARY) exist on disk.
- All 3 task commits (3a57171, 6eac717, f43059c) are ancestors of HEAD.
- Working tree clean at plan end; measured commits from ledger: 3 task commits (docs commit for this SUMMARY follows).
- Verification re-run at plan end: `rfork-header --check` matches; `cargo test -p port rfork` 3/3; `cargo test -p xtask rforkh` 5/5; `cargo test -p xtask --test threat_model` 10/10.
