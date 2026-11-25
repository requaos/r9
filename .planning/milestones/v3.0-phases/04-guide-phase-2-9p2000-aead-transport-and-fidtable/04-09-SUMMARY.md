---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "09"
subsystem: kernel-transport-fidtable
tags: [9p2000-aead, tpht, fid-layout, qid, volatility-class, miri, tdd]

# Dependency graph
requires:
  - phase: 03-guide-phase-1-crypto-and-ds-foundations
    provides: "ds::tpht::{FlattenedTphtShard, TinyPtr, Probe, with_banks_and_probe, deref_tiny}; the DS-03 one-arena-read probe pair"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-01)
    provides: "the wire decisions D-05, D-11 to D-14 and D-27 the layouts implement"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (plan 04-02)
    provides: "port already links crypto/ and ds/, and port/src/lib.rs's per-module forbid style"
provides:
  - "ds::tpht::FlattenedTphtShard::remove zeroes the removed entry's arena words under the block's seqlock"
  - "ds::tpht::FlattenedTphtShard::slot_id and ::read_slot — the dense arena id and the bounds-checked sweep read"
  - "port::fid::{StatefulHandle, StatelessCap, FidHandle, FidEntry, VolatilityClass} with to_wire/from_wire/body/to_words/from_words/from_byte/as_byte"
  - "port::fid::{qid_encode, qid_decode, gpu_qid_path} and STATEFUL_HANDLE_LEN/STATELESS_CAP_LEN/FID_LIVE"
affects: [04-10 fid table, 04-11 single-read resolve, 04-16 sweep, 04-17 test-9p, phase-05 page sealing]

actuals:
  tokens: 11224
  tasks: 2
  commits: 6
  plan_head_before: f2455ef104611209b24b3a1c29006e93229b58e1
  plan_head_after: 08779c7380aae25b79576d1e465afac321eab84a

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A removed TPHT entry's arena slot is zeroed with Release stores inside the block's seqlock section, before the occupied bit is cleared"
    - "The sweep names a slot by its dense index (block << 4) | (ptr - 1) and reads it through a bounds-checked path, never through deref_tiny"
    - "A 64-byte wire-record struct is packed field by field with to_le_bytes, never transmuted, so the module keeps forbid(unsafe_code)"
    - "A variable-length class/enum carried on the wire has its own byte when its values collide with Plan 9 Qid type bits"

key-files:
  created:
    - port/src/fid.rs
    - port/tests/aead_fid.rs
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-09.md
  modified:
    - ds/src/tpht/flattened.rs
    - port/src/lib.rs

key-decisions:
  - "remove zeroes the arena words rather than leaving the header as the only guard: deref_tiny never reads the header, so the stale-slot read had to be closed at the arena (T-04-29, RESEARCH Pitfall 2)"
  - "slot_id is computed through the private split_key + slot_index, so port never duplicates ds's block-index arithmetic; read_slot is the sweep's safe read and is explicitly not the fast path"
  - "FidEntry keeps fid_key and client_id in word 0, which makes the resolve's identity comparison a single atomic word (RESEARCH Pitfall 4)"
  - "the volatility class gets its own FidEntry byte taken from the RFC's reserved area, so dat::Qid is untouched and the three values' collision with QTEXCL/QTAPPEND/QTDIR is pinned by a test (D-27, T-04-30)"
  - "gpu_qid_path keeps the published formula; the u8 node_type mask is dropped because it is an identity operation clippy flags, and the doc states the formula"

patterns-established:
  - "A pin's RED stubs keep the real signatures and wrong bodies, so every RED failure is a behavioural assertion (Pitfall 11, D-03)"
  - "A layout pin mixes hand-written byte vectors with struct-level size/alignment assertions"

requirements-completed: [WIRE-07, WIRE-08, WIRE-10]

coverage:
  - id: D1
    description: "A removed entry's arena slot is zeroed, so deref_tiny of the removed (key, ptr) pair returns all-zero words until the slot is reused"
    requirement: WIRE-08
    verification:
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#tpht_remove_clears_the_arena_slot"
        status: pass
      - kind: unit
        ref: "cargo xtask miri (64 seeds, rc 0, no Undefined Behavior or Data race)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A table can name an arena slot by its dense id and read it with a bounds-checked call that returns None outside the arena"
    requirement: WIRE-08
    verification:
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#tpht_slot_id_is_block_times_sixteen_plus_ptr_minus_one"
        status: pass
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#tpht_read_slot_is_bounds_checked"
        status: pass
      - kind: unit
        ref: "ds/src/tpht/flattened.rs#tpht_deref_tiny_still_one_arena_read"
        status: pass
    human_judgment: false
  - id: D3
    description: "StatefulHandle is 8 bytes, StatelessCap is 24 bytes, both round-trip their wire forms, and a stateful handle is never all zero"
    requirement: WIRE-07
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_layout_sizes_and_alignment"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_layout_wire_forms_roundtrip"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_layout_stateful_handle_is_never_zero"
        status: pass
    human_judgment: false
  - id: D4
    description: "FidHandle::from_wire is Stateful without a capability, Stateless with one and a zero header field, and an error when both are present"
    requirement: WIRE-07
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_layout_capability_requires_zero_header_handle"
        status: pass
    human_judgment: false
  - id: D5
    description: "FidEntry is 64 bytes with 64-byte alignment and round-trips its eight-word form, with fid_key and client_id in word 0"
    requirement: WIRE-07
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_layout_identity_is_word_zero"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_layout_wire_forms_roundtrip"
        status: pass
    human_judgment: false
  - id: D6
    description: "The qid is 13 bytes as type[1] vers[4] path[8]; the volatility class has exactly three bytes that collide with the Plan 9 type bits; the #G path formula holds at its boundaries"
    requirement: WIRE-10
    verification:
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_qid_13_bytes_roundtrip"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_qid_volatility_class_values"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_qid_class_values_collide_with_plan9_type_bits"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_fid_qid_gpu_path_formula"
        status: pass
    human_judgment: false
  - id: D7
    description: "Both pins are recorded RED -> GREEN-1 -> GREEN-2 -> ADVANCED in the phase evidence"
    verification:
      - kind: other
        ref: "grep -cE '^PIN (ds-slot-clear|fid-layouts) (RED|GREEN-1|GREEN-2|ADVANCED)' evidence/PINS-04-09.md == 8"
        status: pass
    human_judgment: false

# Metrics
duration: 12min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 09: `ds` slot clearing and the fid layouts Summary

**`FlattenedTphtShard::remove` now zeroes the removed entry's arena words under the block's seqlock (closing the stale-handle resolve), `slot_id`/`read_slot` give the sweep a dense id and a bounds-checked read, and `port::fid` pins the 8-byte `StatefulHandle`, 24-byte `StatelessCap`, 64-byte `FidEntry`, the three-byte volatility class and the 13-byte qid codec — all in safe code.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-11-25T00:24:50Z
- **Completed:** 2026-11-25T00:37:19Z
- **Tasks:** 2
- **Files modified:** 5 (3 created, 2 modified)
- **Miri:** `MIRI-TIME ds-slot-clear 361` — rc 0, 64 seeds, 33 tpht tests, no `Undefined Behavior`, no `Data race`; over the 120 s feedback-latency budget and **reported, not fixed** by cutting seeds or tests
- **Re-measured unsafe line:** `UNSAFE-LINE flattened.rs:473` (still the only `unsafe` block in `ds/src/tpht/`)

## Accomplishments

- `ds/src/tpht/flattened.rs`: `remove` writes zero words to the entry's arena slot with the existing `Release`-store `write_value`, inside the block's seqlock section and before the occupied bit is cleared. `deref_tiny` never reads the header, so this is the only place the stale-slot resolve could be closed (T-04-29, RESEARCH Pitfall 2).
- `slot_id(key, ptr)` returns the dense arena index `(block << 4) | (ptr - 1)` through the private `split_key`/`slot_index`, and `read_slot(slot_id)` is the sweep's bounds-checked read (`None` outside `0..16 * num_blocks`, `probe.arena_read()` once on a hit). `port` never duplicates `ds`'s block arithmetic (WIRE-09 groundwork).
- `port/src/fid.rs` (new, `#[forbid(unsafe_code)]`): `StatefulHandle` (8 bytes, zero `ptr` rejected), `StatelessCap` (24 bytes plus the 16-byte tagged `body()`), `FidHandle::from_wire` enforcing D-12, `FidEntry` (`repr(C, align(64))`, compile-time size/alignment assertion, `to_words`/`from_words` packing field by field with the identity pair in word 0), `VolatilityClass` (exactly 0x20/0x40/0x80), `qid_encode`/`qid_decode` (type[1] vers[4] path[8]) and `gpu_qid_path`.
- `port/tests/aead_fid.rs` (new): nine pins. Every handle and capability vector is a literal byte array; the qid order and the three class bytes are checked against Harvey `sys/include/libc.h:591-593`.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: the arena-slot and slot-id pin** - `6070190` (test)
2. **Task 1 GREEN: clear the arena slot on `remove`; `slot_id` and `read_slot`** - `3340e25` (feat)
3. **Task 1 ledger: advance `ds-slot-clear`** - `d056bc4` (docs)
4. **Task 2 RED: the fid-layout pin** - `ee11a09` (test)
5. **Task 2 GREEN: `StatefulHandle`, `StatelessCap`, `FidEntry`, `VolatilityClass`, the qid codec** - `0015642` (feat)
6. **Task 2 ledger: advance `fid-layouts`** - `08779c7` (docs)

**Plan metadata:** the SUMMARY commit follows (docs).

## Files Created/Modified

- `ds/src/tpht/flattened.rs` — `remove` zeroes the arena slot; new `pub fn slot_id` and `pub fn read_slot`; module and method docs state the removal guarantee, the sweep read path and the not-a-snapshot property of `deref_tiny`; four new tests.
- `port/src/lib.rs` — `#[forbid(unsafe_code)] pub mod fid;` after `fdt`, with a dated Phase 4 provenance comment. This is the only change to the file.
- `port/src/fid.rs` — the layout half of WIRE-07/WIRE-10: handle, capability, discriminator, entry, class, qid codec and the `#G` path formula.
- `port/tests/aead_fid.rs` — five `aead_fid_layout_*` and four `aead_fid_qid_*` pins.
- `evidence/PINS-04-09.md` — the two pin records (`ds-slot-clear`, `fid-layouts`), the Miri time and the unsafe line.

## Decisions Made

- **Zeroing the arena slot, not just the header.** The header fields were already cleared and could not help: `deref_tiny` resolves from `(key, ptr)` alone with zero header reads, so the stale read had to be closed at the arena. The write reuses `write_value`, so no new `unsafe` and no new ordering rule.
- **`slot_id` goes through `ds`'s own `split_key`/`slot_index`.** The alternative — recomputing `block` in `port` — would duplicate the Feistel/mixed-half layout and could silently diverge from it.
- **`FidEntry` is packed with `to_le_bytes`, never transmuted.** Compendium listing 9.6's `unsafe impl PodSlot` + `transmute` is not copied, so `port::fid` keeps its forbid and the UNSAFE-AUDIT count is unchanged.
- **The volatility class has its own byte** taken from the RFC's `_reserved`, leaving `_reserved: [u8; 15]`. `dat::Qid` is byte-for-byte unchanged, and a test pins that 0x20/0x40/0x80 equal `QTEXCL`/`QTAPPEND`/`QTDIR`.
- **Pin bookkeeping in its own commit.** The LEDGER `ADVANCED` line has to name the GREEN commit, which cannot be inside that same commit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Dropped a no-op `& 0xFF` mask clippy flags in `gpu_qid_path`**
- **Found during:** Task 2 GREEN verification (`cargo xtask clippy`)
- **Issue:** the published formula's `(node_type & 0xFF) as u64` is an identity operation on a `u8`, and `clippy::identity_op` warned at `port/src/fid.rs:384`, naming the plan's own formula.
- **Fix:** `| node_type as u64`, with a comment stating that the mask is a no-op on a `u8` and that the doc comment still carries the published formula. The `ctx_id & 0x00FF_FFFF` mask is real and stays.
- **Files modified:** `port/src/fid.rs`
- **Verification:** `cargo xtask clippy` exits 0 and no warning names `port/src/fid.rs`; `aead_fid_qid_gpu_path_formula` still passes all four boundary cases.
- **Committed in:** `0015642` (Task 2 feat commit)

### Process deviations

**2. Two extra `docs` commits for the pin `ADVANCED` lines** (`d056bc4`, `08779c7`)
- The plan names two commits per task; four in total. Each pin's `ADVANCED` line must name the GREEN commit, so it cannot ride inside that commit. The 04-02 precedent is the same: a `docs` commit advances the pin. Net effect is 6 commits, no extra code.

---

**Total deviations:** 1 auto-fixed (1 bug), 1 process note.
**Impact on plan:** The clippy fix is a one-token change to a pure function with identical behaviour. The extra docs commits carry only ledger lines.

## Issues Encountered

- **The worktree path guard reported a sibling worktree's cwd.** The first `Edit` against an absolute path was blocked, and the guard's diagnostic named `04-17`'s worktree as the active root — a stale `cwd` in the harness's tool payload while several executors share one process. Relative paths are used for every `Edit`/`Write` thereafter, and each write was verified to land in this worktree (`04-09`) by `ls` and `git status`. No file was written to the main checkout or to `04-17`.
- **The first Task 1 commit carried a `Co-authored-by` line.** It was removed with `git commit --amend` before anything else touched the branch; `git log` has no attribution line on any of the six commits. Recorded here so the amendment is on the record rather than discovered in the reflog.

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None. Every public function in `port::fid` and both new `ds` methods has its real body; the RED stubs (zero wire forms, `Err`, zero words) were all replaced in the GREEN commits.

## Threat Flags

None beyond the plan's `<threat_model>`. The new surface is the same client→handle boundary T-04-29/T-04-30/T-04-31 already register, and each is now covered by a pin.

## Next Phase Readiness

- **WIRE-07 and WIRE-10 are implemented; WIRE-08 is groundwork only.** The handle/capability/entry/qid layouts and the class byte are done. WIRE-08's table, shards, single-read resolve and the sweep are later plans (04-10, 04-11, 04-16); this plan supplies the zeroed slot, `slot_id`/`read_slot` and `FID_LIVE` that they stand on. The plan frontmatter declares WIRE-08 and it is copied verbatim into `requirements-completed`, so treat the requirement's bookkeeping at phase verification rather than as "WIRE-08 complete".
- **`UNSAFE-AUDIT.md`'s `ds/src/tpht/flattened.rs:440` is now stale** — the one block is at line 473. Plan 04-12 owns that paragraph.
- **`port/src/fid.rs` is library code that nothing reachable calls yet**, so it links no kernel code by itself (RESEARCH Pitfall 8); the kernel-byte change still comes from the dependency edge and the `devmnt` branch.
- **Miri is again over the 120 s budget** (361 s, all 33 tests × 64 seeds, rc 0). This joins the Phase 3 overages already carried to the phase-end review; no seed or test was cut.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- All three created files exist on disk (`port/src/fid.rs`, `port/tests/aead_fid.rs`, `evidence/PINS-04-09.md`).
- All six commits are ancestors of HEAD (`6070190`, `3340e25`, `d056bc4`, `ee11a09`, `0015642`, `08779c7`).
- The four new tpht tests report `... ok` (4/4) and the nine `aead_fid_layout_*` / `aead_fid_qid_*` tests report `... ok` (9/9).
- The pin ledger carries 8 `PIN (ds-slot-clear|fid-layouts) (RED|GREEN-1|GREEN-2|ADVANCED)` lines and 2 `MIRI-TIME`/`UNSAFE-LINE` lines.
- `cargo xtask test` ×2, `cargo xtask check`, `cargo xtask clippy` and `cargo xtask miri` all exit 0; `port/src/dat.rs` is unchanged against the plan base.
