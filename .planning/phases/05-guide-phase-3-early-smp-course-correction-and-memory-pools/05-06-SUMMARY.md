---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 06
subsystem: port
tags: [fid, slab, seqlock, 9p2000-aead, corr-03, corr-05, tdd, owner-stop, unsafe-policy, rfc-amendment]

# Dependency graph
requires:
  - phase: 05-02
    provides: CORR-04 rfork retirement and the v3.0 threat-model rescope this plan's docs sit beside
  - phase: 05-04
    provides: CORR-05's lint opt-in and reconciled UNSAFE-AUDIT.md whose policy sentence this plan syncs everywhere else
  - phase: 05-05
    provides: ds/src/slab.rs — the #![forbid(unsafe_code)] Boehm seqlock slab (SlabSlots<7, P>, AllocMeta, SlabProbe/NoSlabProbe, never-zero generations) that port/src/fid.rs wraps
provides:
  - port/src/fid.rs: slab-backed FidTable under #[forbid(unsafe_code)] — server-assigned StatefulHandle (slot_idx(2) gen(2) shard_id(1) flags(1) epoch(2) LE, gen >= 1, from_wire rejects gen == 0), 56-byte FidEntry in 7 slot words, sizing tiers (synthetic/default_workstation/with_tier), shard_count_for_cpus, round-robin spillover from CPU 0, lock-free resolve with the D-08 owner check kept, update/clunk/mark_reclaimed/sweep_unreclaimed serialized per shard
  - port/tests/aead_fid.rs: 31 pins — wire codec, CountingSlabProbe 1-slot-read, spillover, tiers, owner enforcement on every mutator, threaded torn-snapshot pin, sweep set model
  - port/tests/common/aead_link.rs: E2eServer on FidTable::synthetic with a shared-table variant (connect_aead_shared / E2eServer::new_shared) modeling one-table-per-server
  - D-13 three-place amendment: rfc-9p2000-aead.md §5 dated note, PROJECT.md Key Decisions rows, compendium v3.0 §4 divergence note (REQUIREMENTS.md CORR-03 is the live requirement)
  - CORR-05 identical policy wording across Cargo.toml, the v3.0 guide, the v3.0 compendium (§1 row + §2 tree) and UNSAFE-AUDIT.md
affects: [05-07 lock ranks (FidShard meta is a named rank-40 candidate), 05-08 port::smp current_cpu_id wiring (current_cpu_hint seam), 05-27/D-28 in-kernel loopback server (synthetic tier), phase-6 FidTable consumers, Owner Stop 1]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (55000, confidence: low).
actuals:
  tokens: 44560
  tasks: 3
  commits: 5
  plan_head_before: c0c584b5f5009e54e0b612742600b148e5bd3ff5
  plan_head_after: 3799f66a7ecdc0d47216d5564f97fb71380c1b20

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "FidShard<P: SlabProbe>: #[repr(align(64))] pairing SlabSlots<7, P> with Lock<AllocMeta> — resolve touches only the slot array (1 cache-line read, 0 locks); every writer serializes through shard.meta so no two writers race on WRITING (D-11)"
    - "Server-assigned handles: alloc routes current_cpu_hint() & shard_mask (CPU 0 until port::smp lands in 05-08) with round-robin fallover; no lookup by client fid number, no duplicate-fid check, no all-shard probe, no table-wide insert lock (D-07, D-12)"
    - "D-08 owner check under the writer lock: update/clunk/mark_reclaimed resolve + compare entry.client_id inside the meta-lock critical section, so check-and-act is atomic; update forces client_id/epoch back after the closure"
    - "r#gen raw-identifier spelling for the D-10 wire field: gen is a reserved keyword on the pinned edition-2024 toolchain (gen blocks); locals named generation"

key-files:
  created: []
  modified:
    - port/src/fid.rs
    - port/tests/aead_fid.rs
    - port/tests/common/aead_link.rs
    - port/tests/aead_negotiate.rs
    - port/src/lib.rs
    - port/src/chan/wire.rs
    - Cargo.toml
    - .planning/rfc-9p2000-aead.md
    - .planning/PROJECT.md
    - docs/plan_r9_architecture_code_compendium_v3_0.md
    - docs/r9_implementation_guide_for_agentic_developers_v3_0.md
    - .planning/REQUIREMENTS.md
    - .planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-06-RED-task1.json

key-decisions:
  - "FidEntry shrunk to 56 bytes (49 live bytes + 7 reserved) packed into the slot's 7 payload words: FID_LIVE dropped (slot OCCUPIED replaces it, D-11), fid_key kept as a record only (D-07), align(64) dropped — the slot, not the entry, is the cache-line unit"
  - "Sizing tiers: synthetic() = 4x256, default_workstation() = 8x1024, with_tier() enforces the cluster ranges 4096..=16384 x 16..=32; the general new() validates power-of-two shapes because alloc routes by shard mask (D-12)"
  - "The aead_negotiate cross-connection refusal pin moved to the shared-table fixture: under server-assigned handles two private tables assign identical (shard 0, slot 0, gen 1) values, so the per-connection-table fixture could not express T-05-09; one table with two client_ids can (D-08 end to end)"

patterns-established:
  - "RED-for-a-refactor shape: full new implementation committed as test() scaffolding with exactly the planned security guards omitted (8 planned assertion failures, 23 unrelated pins green), GREEN adds the guards — mirrors 05-05 and keeps INVALID_RED (build failure) out of a wholesale rewrite"
  - "CountingSlabProbe against the public ds::slab::SlabProbe trait from port/tests (ds's own probe is test-private) — the WIRE-08 one-cache-line pin restated for the slab"
  - "Dated doc-correction markers inside owner v3.0 documents ('Corrected 2026-11-25 (owner, Phase 5 D-02): ...') so the corrected wording and its provenance travel together"

requirements-completed: [CORR-03, CORR-05]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "port/src/fid.rs wraps ds::slab under #[forbid(unsafe_code)]: zero FlattenedTphtShard, zero insert_lock, zero all-shard probe; server-assigned StatefulHandle with gen >= 1 and gen == 0 rejected on the wire (CORR-03, D-07, D-09, D-10)"
    requirement: CORR-03
    verification:
      - kind: automated
        ref: "grep -v '^[[:space:]]*//' port/src/fid.rs | grep -c 'FlattenedTphtShard' == 0; nix develop -c cargo test -p port --test aead_fid (31 passed; aead_fid_layout_stateful_handle_is_never_zero)"
        status: pass
    human_judgment: false
  - id: D2
    description: "resolve takes zero locks and one cache-line slot read (CountingSlabProbe), rejects client_id == 0, foreign client_id, stale gen, wrong epoch, out-of-range shard/slot; clunk scrubs and bumps gen; update keeps the wire handle under the shard meta lock; round-robin spillover never fails on clustering (CORR-03, D-08, D-11, D-12)"
    requirement: CORR-03
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs: aead_fid_resolve_is_one_slot_read_and_zero_locks, aead_fid_resolve_rejects_wrong_client_epoch_and_shard, aead_fid_alloc_refuses_client_id_zero, aead_fid_update_keeps_the_wire_handle_and_enforces_the_owner, aead_fid_update_in_place_never_resolves_a_torn_snapshot, aead_fid_clunk_scrubs_recycles_and_enforces_the_owner, aead_fid_alloc_routes_cpu_zero_and_spills_over_round_robin — all pass"
        status: pass
      - kind: integration
        ref: "nix develop -c cargo xtask test-9p — all 14 legs green twice consecutively (IG-V01), incl. aead_negotiate_end_to_end_handle_from_another_connection_is_refused on the shared table"
        status: pass
    human_judgment: false
  - id: D3
    description: "D-13 three-place amendment notes (rfc §5, PROJECT.md Key Decisions, compendium §4) and identical CORR-05 unsafe-policy wording across Cargo.toml, the v3.0 guide, the compendium (§1, §2) and UNSAFE-AUDIT.md (CORR-03, CORR-05, D-02, D-13)"
    requirement: CORR-05
    verification:
      - kind: automated
        ref: "dated-marker greps (1 hit per file); no stale normative 'crate-wide port/' wording; cargo xtask check rc 0; cargo test -p xtask --test threat_model 10 passed; full cargo xtask test rc 0 (56 ok legs) + cargo xtask miri rc 0 at final state"
        status: pass
    human_judgment: false
  - id: D4
    description: "Owner Stop 1 (D-30, D-32): cumulative port/src diff (0d08e6c..HEAD: 4 files, +396/-489), unpushed commit list (origin/main = 900f58b) and host/clippy/Miri evidence presented for owner approval; push to origin/main only on approval so CI proves main green and pure boot-iso"
    requirement: CORR-03
    verification:
      - kind: human
        ref: "Owner Stop 1 review, 2026-11-26: owner reviewed the cumulative port/src diff (0d08e6c..HEAD: 4 files, +396/-489), the unpushed commit list and the host/clippy/Miri evidence; response: 'approved, push'"
        status: pass
    human_judgment: true
    rationale: "The stop exists so the owner reviews the cumulative port/src diff and explicitly approves any push (D-30, D-32); no automation can substitute for the owner's verdict. Verdict rendered 2026-11-26: approved; push to origin/main authorized; CI on main is the remaining external check (CORR-01, CORR-02, D-05)."

# Metrics
duration: 26min
completed: 2026-11-26
status: complete
phase_status: complete (3 of 3 tasks executed; Owner Stop 1 approved by the owner 2026-11-26 with push to origin/main authorized)
---

# Phase 05 Plan 06: Slab FidTable, D-13 Amendments and Owner Stop 1 Summary

**`port/src/fid.rs` now runs on the `ds::slab` Boehm seqlock slab under `#[forbid(unsafe_code)]` — server-assigned `StatefulHandle` (`slot_idx(2) gen(2) shard_id(1) flags(1) epoch(2)` LE, gen never 0), a 56-byte `FidEntry` in 7 slot words, lock-free 1-cache-line `resolve` with the owning-`client_id` check kept, D-12 sizing tiers with round-robin spillover, and zero TPHT/`insert_lock`/all-shard probe remaining — with the D-13 three-place amendments recorded, the CORR-05 policy sentence identical in all four places, all 14 `test-9p` legs green twice, and the full host suite (check, both arch clippy legs, test, Miri) green at the final state for Owner Stop 1.**

## Performance

- **Duration:** 26 min
- **Started:** 2026-11-26T05:20:08Z
- **Completed:** 2026-11-26T05:46:10Z (tasks 1-2); 2026-11-26 (Task 3 — Owner Stop 1 approved by the owner)
- **Tasks:** 3 of 3 executed (Task 3 was the Owner Stop 1 `checkpoint:human-verify`, gate=blocking — PASSED on owner approval per D-30/D-32)
- **Files modified:** 12 (0 created; plus the RED evidence record JSON)

## Accomplishments

- **CORR-03 — the FidTable is a slab.** `port/src/fid.rs` wraps `ds::slab`: `#[repr(align(64))] FidShard<P>` pairs `SlabSlots<7, P>` with `Lock<AllocMeta>`; `resolve(client_id, handle)` takes zero locks, performs exactly one slot read (`CountingSlabProbe` pin) and returns a consistent seqlock snapshot; writers (`alloc`, `update`, `clunk`, `mark_reclaimed`, `sweep_unreclaimed`) serialize through the shard-local meta lock so no two writers race on `WRITING`. `FlattenedTphtShard`, the table-wide `insert_lock` of `6fd201c`, the all-shard duplicate probe and `FID_LIVE` are all gone (grep-verified zero non-comment references).
- **CORR-03 — the wire handle is the locked D-10 layout.** `StatefulHandle` is server-assigned: `slot_idx(2) gen(2) shard_id(1) flags(1) epoch(2)` little-endian; `from_wire` rejects `gen == 0` (including `[0u8; 8]`), so a stateful handle is never all zero and the Phase 4 D-12 discriminator is re-proved without the TinyPtr byte. Alloc routes from `current_cpu_hint() & shard_mask` (CPU 0 until `port::smp` lands in 05-08) and falls over round-robin; sizing tiers `synthetic()` (4×256), `default_workstation()` (8×1024), `with_tier()` (cluster 4,096..=16,384 × 16..=32) and `shard_count_for_cpus()` are pinned.
- **CORR-03 — the owning-client check is kept end to end (D-08, T-05-09).** `resolve`/`update`/`clunk`/`mark_reclaimed` refuse `client_id == 0` and compare `entry.client_id == client_id` (writers do check-and-act atomically under the meta lock); `update` preserves the wire handle across in-place writes and forces the identity back after the closure. The `aead_negotiate` refusal pin now runs over **one shared table** with two client_ids — the real one-table-per-server deployment — and still answers `Rerror` to the foreign handle.
- **CORR-03/CORR-05 — the amendment and policy trail is written.** RFC §5 carries the dated `2026-11-25 — CORR-03 / Phase 5 D-07..D-13` note superseding the archived WIRE-07..09 TPHT wording; `PROJECT.md` Key Decisions gained the two dated rows (superseded wording + CORR-05 floor-module sync); the compendium v3.0 §4 carries the dated two-divergences note (lock-free seqlock slot; kept client check); the CORR-05 sentence — `crypto/` crate-wide, `ds/` per-module (`tpht/` + one reserved Roaring SIMD module), `port/` per-module (`globalalloc`, `mcslock`, `devip_waker`), deny lint on `ds/src/tpht/` + both arch crates — now reads identically in `Cargo.toml`, the v3.0 guide (two spots, with dated correction markers), the compendium (§1 row and §2 tree) and `UNSAFE-AUDIT.md`.
- **Verification evidence for Owner Stop 1.** `cargo xtask check` rc 0; `cargo xtask clippy --arch aarch64 --config virtshell` rc 0; `cargo xtask clippy --arch x86-64 --config q35cpu` rc 0; `cargo xtask test` rc 0 (56 `test result: ok` legs) at the final state; `cargo xtask miri` rc 0; `cargo xtask test-9p` all 14 legs green twice consecutively (IG-V01). REQUIREMENTS.md: CORR-03 and CORR-05 marked Complete.

## Task Commits

Task 1 ran TDD (RED before GREEN, IG-V01 GREEN twice); Task 2 is a docs task:

1. **Task 1, RED: failing slab FidTable pins with scaffolding (CORR-03)** — `bcae5dc` (test) — 23 passed, 8 failed on the planned D-08/D-10 security assertions; RED evidence `05-06-RED-task1.json` classified `RED_EVIDENCE_OK` (`target_test_failed`)
2. **Task 1, GREEN: slab-backed FidTable with D-08 owner checks and D-10 gen guards (CORR-03)** — `0de7b42` (feat) — 31/31 aead_fid pins pass; all 14 test-9p legs green twice; full `cargo test -p port` green (168 lib + all integration files)
3. **Task 2: D-13 three-place amendments and CORR-05 policy sync (CORR-03, CORR-05)** — `c1b02e5` (docs) — full host suite green at final state (check, both arch clippy legs, test, Miri)
4. **Task 3: Owner Stop 1 — Course Correction & Audit Follow-Up Review (D-30, D-32)** — PASSED — owner reviewed the cumulative `port/src` diff (`0d08e6c..HEAD`: 4 files, +396/-489), the unpushed commit list and the host/clippy/Miri evidence, and responded "approved, push" on 2026-11-26

**Plan metadata:** the SUMMARY + REQUIREMENTS.md commit (docs: complete plan), committed atomically with this file; the follow-up docs commit recording the Owner Stop 1 approval.

## Owner Stop 1 Outcome

**Approved by the owner on 2026-11-26; push to `origin/main` authorized.** The owner reviewed the cumulative `port/src` diff (`0d08e6c..HEAD`: `chan/wire.rs`, `fid.rs`, `lib.rs`, `rfork.rs`; +396/-489), the unpushed commit list from `900f58b..HEAD`, and the host evidence (check, both arch clippy legs, full test suite, Miri, 14 `test-9p` legs green twice), and answered "approved, push". CI on `main` — green with pure `boot-iso` — is the remaining external check (CORR-01, CORR-02, D-05). The push itself is deferred to the orchestrator.

## Files Created/Modified

- `port/src/fid.rs` — **rewritten** (939 → ~800 lines): module docs restated for the slab; `StatefulHandle` (new fields/wire order, `r#gen` spelling); `FidEntry` 56-byte 7-word codec without `FID_LIVE`; `FidShard<P>` = `SlabSlots<7, P>` + `Lock<AllocMeta>`; `FidTable` with tiers, `alloc`, lock-free `resolve`, `update`, `clunk`, `mark_reclaimed`, `sweep_unreclaimed`, `bitmap_lens`, unchanged `set_epoch`/`issue_cap`/`verify_cap`; `StatelessCap`, `FidHandle`, `VolatilityClass`, `qid_encode`/`qid_decode`, `gpu_qid_path` untouched
- `port/tests/aead_fid.rs` — **rewritten** (1,563 → ~830 lines): 31 pins; TPHT-era tests (duplicate-refused, home_shard/fallover, racing-pair, multi-block model, torn-image, sweep-window probe) replaced by slab pins (server-assigned distinct handles, round-robin spillover, tiers, owner enforcement, threaded torn-snapshot pin, sweep set model on `(shard_id, slot_idx)`)
- `port/tests/common/aead_link.rs` — `E2eServer` holds `Rc<RefCell<FidTable>>` with `new` (private synthetic table) and `new_shared` (one-table-per-server) constructors; `connect_aead_shared`/`connect_aead_shared_with` helpers; `connect_aead_with`'s body factored into `connect_aead_over`; `insert` → `alloc`; `fid_memory` walks `shard_count()`
- `port/tests/aead_negotiate.rs` — the cross-connection refusal pin shares one table between two client_ids and additionally proves B's own handle still resolves
- `port/src/lib.rs`, `port/src/chan/wire.rs` — module/wire comments track the slab layout and the D-10 never-zero-generation discriminator
- `Cargo.toml` — header comment now states the full CORR-05 policy sentence
- `.planning/rfc-9p2000-aead.md` — §5 dated CORR-03 amendment note (after the 2026-11-24 note)
- `.planning/PROJECT.md` — two dated Key Decisions rows (CORR-03 three-place record; CORR-05 sync)
- `docs/plan_r9_architecture_code_compendium_v3_0.md` — §4 dated divergence note; §2 crate tree + §1 `ds/` row corrected to per-module `port/` with the floor modules
- `docs/r9_implementation_guide_for_agentic_developers_v3_0.md` — task 5 and operating-rule-2 corrected per D-02 with dated markers
- `.planning/REQUIREMENTS.md` — CORR-03 and CORR-05 marked complete (checkboxes + traceability table)
- `05-06-RED-task1.json` — **created** (TDD RED evidence record, committed with the RED commit)

## Decisions Made

- **`FidEntry` is 56 bytes, not 64.** The slot's eighth word is the seqlock state, so the entry packs its 49 live bytes plus 7 reserved into `[u64; 7]`; `#[repr(C, align(64))]` moved off the entry (the slot is the cache-line unit). `fid_key` stays as a record only (D-07); `FID_LIVE` is gone (slot `OCCUPIED` replaces it, D-11). This is the "Claude's Discretion" call 05-CONTEXT.md leaves open (keep `fid_key`/`epoch` vs drop them) — kept both, 7 spare bytes.
- **The shared-table fixture is the honest shape for T-05-09.** With server-assigned handles, two private tables alias `(shard 0, slot 0, gen 1)` on their first allocs, so the old two-table refusal test could not fail for the right reason; one table with two client_ids exercises the D-08 owner check end to end and matches the compendium's per-server sizing.
- **`r#gen` over renaming the field.** The wire layout's name for the field is `gen` (locked D-10); `gen` is a reserved keyword on the pinned toolchain (edition-2024 gen blocks, the same finding 05-05 recorded for locals), so the field is spelled `r#gen` and locals are `generation`.
- **Tier constructors are infallible, `new`/`with_tier`/`with_probe` fallible.** The named tiers' constants are valid by construction (power-of-two shapes); the general constructors validate because alloc routes by shard mask and the slab requires power-of-two slot counts.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Re-modeled the cross-connection refusal pin onto one shared table**
- **Found during:** Task 1 GREEN verification (`cargo xtask test-9p`: `aead_negotiate_end_to_end_handle_from_another_connection_is_refused` FAILED — "a handle issued on another connection does not resolve")
- **Issue:** The Phase 4 test gave each connection its own private `FidTable`. Under the locked slab design (server-assigned `(shard, slot, gen)` handles, one table per server per the D-12 sizing), two fresh tables assign identical first handles, so A's handle aliased B's own slot-0 entry owned by B's client — the D-08 check passed correctly and the old assertion could never hold for the right reason. The test encoded the TPHT-era mechanism (handle carries `fid_key`), not the threat.
- **Fix:** `E2eServer` now holds `Rc<RefCell<FidTable>>` with a `new_shared` constructor and `connect_aead_shared` helpers; the test shares one table between client_ids 1 and 2, so the refusal is the real T-05-09 owner check (`entry.client_id != connection client_id` → `Rerror`), and a second assertion proves B's own handle still resolves on the shared table. The D-08 owner verification on `resolve`/`clunk`/`mark_reclaimed` (the plan's `resolved` prohibition) is untouched and pinned at both the unit and the wire level.
- **Files modified:** `port/tests/common/aead_link.rs`, `port/tests/aead_negotiate.rs`
- **Verification:** `cargo test -p port --test aead_negotiate` 11/11; full `cargo xtask test-9p` 14 legs green twice
- **Committed in:** `0de7b42` (Task 1 GREEN commit)

**2. [Rule 1 - Bug] Corrected the missed stale "crate-wide port/" instance in compendium §1**
- **Found during:** Task 2 acceptance-criteria scan (my own stale-wording grep caught it)
- **Issue:** The plan's action item named the compendium §2 tree for the CORR-05 sync, but the §1 impact table's `ds/` Unsafe Policy row also stated "`crypto/` and `port/` are crate-wide `#![forbid(unsafe_code)]`" — contradicting D-02 and the plan's acceptance criterion that the policy reads identically everywhere.
- **Fix:** Corrected the §1 row in place with the same dated correction marker style used in the guide.
- **Files modified:** `docs/plan_r9_architecture_code_compendium_v3_0.md`
- **Verification:** stale-wording scan clean (`no stale normative crate-wide port wording`); `cargo test -p xtask --test threat_model` 10 passed (it reads the compendium); `cargo xtask check` rc 0
- **Committed in:** `c1b02e5` (Task 2 commit)

**3. [Rule 3 - Blocking] Spelled the wire field `r#gen` for the pinned toolchain**
- **Found during:** Task 1 RED first compile — `gen` is a reserved keyword on nightly-2026-01-30 (edition-2024 gen blocks); the struct definition, initializers and accessors failed to parse (05-05 hit the same for locals)
- **Issue:** Blocking compile failure on the D-10 field name.
- **Fix:** Field spelled `r#gen` (keeps the locked wire-layout name), locals renamed `generation`; documented on the field.
- **Files modified:** `port/src/fid.rs`, `port/tests/aead_fid.rs`
- **Verification:** RED evidence run compiled and produced the planned 8 failures / 23 passes
- **Committed in:** `bcae5dc` (Task 1 RED commit)

---

**Total deviations:** 3 auto-fixed (2 bugs — one test-model, one doc wording; 1 blocking)
**Impact on plan:** Each fix is required for the plan's own acceptance criteria to mean what they say (the refusal pin testing the actual D-08 threat, the policy wording being identical in *all* places, the code compiling at all). No scope creep; no public API beyond the plan's artifact list; `Cargo.lock` unchanged (zero external packages — T-05-SC gate N/A).

## Issues Encountered

- None beyond the deviations above. Baseline suite was green at spawn; no pre-existing failures were touched (scope boundary respected: the pre-existing port clippy warnings, e.g. `devip.rs` unused import, were left alone).

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Every artifact is fully wired: the table is exercised by 31 pins plus the `aead_negotiate` end-to-end legs; the docs carry the dated notes; `Cargo.lock` is untouched. The broken-windows ledger (`.planning/WINDOWS.md`) has no entries to add for this plan: no stubs, no skipped tests, no unrun `<verify>`; the three deviations were fixed and verified in-commit (nothing left open to register).

## Threat Flags

None new. The plan's register is implemented as written: T-05-09 (owner `client_id` verification on `resolve`/`update`/`clunk`/`mark_reclaimed`) and T-05-10 (`gen == 0` rejection, epoch check, shard/slot bounds, payload scrub on clunk) are both `mitigate` and both pinned by the new tests; the trust boundary (untrusted handle bytes over the 9P session) is unchanged. The new public surface is exactly the plan's artifact list (`update` is named in it); the wire layout is the locked D-10 layout.

## Next Phase Readiness

- **Owner Stop 1 (Task 3) PASSED:** the owner approved the cumulative `port/src` diff since phase base `0d08e6c` (4 files: `chan/wire.rs`, `fid.rs`, `lib.rs`, `rfork.rs`; +396/-489), the unpushed commits from `900f58b..HEAD`, and the host evidence on 2026-11-26, authorizing the push to `origin/main`. CI now proves `main` green with pure `boot-iso` (CORR-01/CORR-02, D-05) as the remaining external check before Part 2 (Plans 05-07+) starts.
- **Plan 05-07:** `FidShard.meta` is the `Lock<AllocMeta>` that takes a rank (the plan notes Rank 40) once lock ranks land; the slab itself needs no change.
- **Plan 05-08:** `current_cpu_hint()` in `port/src/fid.rs` is the single seam to rewire to `port::smp::current_cpu_id()`; the D-12 routing tests pin CPU-0 behavior until then.
- **D-28 loopback server (05-27):** the synthetic tier (4×256) is the sizing the gate wiring constructs; `connect_aead_shared` is the test-side precedent for one table behind two ends.

## TDD Gate Compliance

Task 1 is `tdd="true"` and followed RED → GREEN with a committed failing pin before implementation (Task 2 is `type="auto"` without the attribute and produced one docs commit):

| Task | RED commit | GREEN commit | RED evidence (failing assertion, not compile error) |
|------|-----------|--------------|-----------------------------------------------------|
| 1 (slab FidTable) | `bcae5dc` | `0de7b42` | `RED_EVIDENCE_OK` (`target_test_failed`) over `05-06-RED-task1.json`: 8 planned failures — `aead_fid_layout_stateful_handle_is_never_zero` (gen == 0 accepted by the scaffolding), `aead_fid_layout_capability_requires_zero_header_handle` (zero header + no cap accepted), `aead_fid_alloc_refuses_client_id_zero`, `aead_fid_resolve_rejects_wrong_client_epoch_and_shard`, `aead_fid_resolve_two_clients_same_fid_number`, `aead_fid_update_...`, `aead_fid_clunk_...`, `aead_fid_mark_reclaimed_enforces_the_owner` (D-08 owner checks absent) — 23 unrelated pins passed in the same binary |

Semantic assessment (recorded in the RED JSON): every failing test executed and panicked on its planned D-08/D-10 security assertion — no compile errors, no zero-test discovery, no fixture crashes. GREEN (`0de7b42`) descends from RED (`bcae5dc`) in commit ancestry. No REFACTOR commit was needed; the GREEN implementation was final-form. `workflow.tdd_mode` is `false` in this project, so the plan-level gate is informational here; the task-level `tdd="true"` cycle was followed with machine-classified RED evidence.

---

*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26 (3 of 3 tasks; Owner Stop 1 approved by the owner, push to origin/main authorized)*

## Self-Check: PASSED

- All 12 created/modified files exist on disk (verified with `[ -f ]`): `port/src/fid.rs`, `port/tests/aead_fid.rs`, `port/tests/common/aead_link.rs`, `port/tests/aead_negotiate.rs`, `port/src/lib.rs`, `port/src/chan/wire.rs`, `Cargo.toml`, `.planning/rfc-9p2000-aead.md`, `.planning/PROJECT.md`, `docs/plan_r9_architecture_code_compendium_v3_0.md`, `docs/r9_implementation_guide_for_agentic_developers_v3_0.md`, `.planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-06-RED-task1.json` (plus `.planning/REQUIREMENTS.md` marked complete, committed beside this SUMMARY).
- All 4 prior plan commits are ancestors of HEAD: `bcae5dc` (RED), `0de7b42` (GREEN), `c1b02e5` (docs), `3799f66` (plan metadata).
- Content pins verified: `SlabSlots<FID_ENTRY_WORDS, P>` and `r#gen: u16` in `port/src/fid.rs`; `connect_aead_shared` in both test files; zero `use ds::tpht` / `insert_lock` / `FlattenedTphtShard` in `port/src/fid.rs`.
- Measured commits from ledger (`gsd-plan-head-before-05-06` = `c0c584b5`): 5 at completion (3 task commits + 2 docs/metadata commits); `plan_head_after` = `3799f66a` (last commit before this update's own docs commit).
- Working tree before this SUMMARY's commit: only this SUMMARY untracked and `REQUIREMENTS.md` modified (both staged in the metadata commit); zero `unsafe` tokens added anywhere; `Cargo.lock` untouched; `ds/src/tpht/` untouched.
- Plan-level verification re-run at completion: `cargo xtask test-9p` rc 0 twice ✓, `cargo test -p port fid` green ✓, `cargo xtask check` rc 0 ✓, both arch clippy legs rc 0 ✓, `cargo xtask test` rc 0 (56 ok legs) ✓, `cargo xtask miri` rc 0 ✓.
