---
phase: 07-r9-pci-ecam-virtio-net-devether
plan: 03
subsystem: infra
tags: [r9, virtio, virtio-net, pci, virtqueue, feature-negotiation, memory-barriers, aarch64, tdd, unsafe-free, forbid-unsafe-code]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration
    provides: port::allocator index/arena idiom + allocator_tests crate-root placement precedent (the shapes this plan's ring model and test module copy); the boot-test harness the transport's MMIO edges will gate through in 07-06
  - phase: 07-r9-pci-ecam-virtio-net-devether (plan 07-01)
    provides: the always-on idle virtio-net NIC on every aarch64 virt qemu invocation (D-09/D-10) — the device this transport binds in 07-06
provides:
  - port::virtio — unsafe-free virtio 1.x modern transport (#[forbid(unsafe_code)]): Caps capability model (§4.1.4), the §3.1.1 negotiation machine with the D-03 zero-offload mask (exactly MAC|STATUS|VERSION_1, loud VERSION_1-absent/FEATURES_OK rejects leaving the device FAILED), queue-setup helpers (queue_enable LAST, DRIVER_OK before notify), split-virtqueue ring index model (descriptor pool, 2-desc chains, avail pre-link, barrier-sandwich post, acquire-ordered used-drain with V5 id/len validation), 12-byte virtio_net_hdr model with const size assert
  - port::virtio_tests — 21 crate-root host tests over Vec-backed mock arenas with a shared op-order log (barrier placement and §3.1.1 step order asserted as SEQUENCES)
  - HowItWorks.md virtio section — the D-01 provenance record (NETDRV-02): from-scratch choice, control/audit rationale, virtio-drivers 2025-09 stale-caveat note, costly reversibility rating
affects: [07-04, 07-05, 07-06, 07-07, 07-08, phase-8, phase-11]

# Actuals (#2632) — pairs with the plan's estimate (tokens 60000, confidence: low).
actuals:
  tokens: 17375     # chars/4 over the realized r9 diff (aaedaa7..bf11fe4, 69501 chars)
  tasks: 2
  commits: 4        # MEASURED: git rev-list --count aaedaa7..bf11fe4 (r9); the harvey metadata docs commit rides on top

plan_head_before: aaedaa751359b92a28734c83844b1eb70d49a52a   # r9 repo (code commits)
plan_head_after: bf11fe478fe81002283ad794e198e5b3913a0de6    # r9 repo

# Tech tracking
tech-stack:
  added: []   # zero new dependencies (T-07-SC: the Cargo diff is empty — verified by git diff --stat, 4 files all additive)
  patterns:
    - "RingMem/CommonCfg/PciCfg volatile seams with test-mock op-order logging: barrier placement and spec step order are asserted as operation SEQUENCES, not outcomes (the tracer's mock-recording backend pattern)"
    - "The allocator.rs index/arena idiom re-expressed for virtqueues: descriptor index pool + per-slot chain-head side table + posted[] ownership, no intrusive pointers in device-visible memory"
    - "fence(order) as a RingMem trait method whose default lowers to core::sync::atomic::fence (dmb ish / dmb ishld on aarch64) — the mock overrides to record position; zero new asm!, zero unsafe"

key-files:
  created:
    - r9:port/src/virtio.rs
    - r9:port/src/virtio_tests.rs
  modified:
    - r9:port/src/lib.rs
    - r9:HowItWorks.md

key-decisions:
  - "07-03: the RingMem trait carries fence(order) with a default that calls core::sync::atomic::fence — Release→dmb ish / Acquire→dmb ishld per the RESEARCH-verified codegen — so production ring code reads as spec-ordered volatile ops and the test mock overrides fence to record op ORDER (the barrier sandwich is test-proven, not prose)"
  - "07-03: avail pre-linking is literal — Ring::new writes every avail entry to its slot's chain head once, and post re-writes the same fixed value as part of the §2.7.13 publication sequence (ethervirtio.c's availent shadow made concrete); a fresh ascending descriptor pool reproduces the C's 2i chain-head layout organically"
  - "07-03: queue-size ceiling MAX_QSIZE=256 with loud construction failure beyond it (the EDGE-PROBE assumption: accept qemu's 256 as-offered, never downsize — a larger offer is a bind error, never a truncated side table)"
  - "07-03: used-ring id validation rejects odd ids as device faults (beyond the plan's id<qsize: chain heads are even — an odd id would alias the wrong slot); len bounds per direction (RX 12..=capacity, TX informational)"

patterns-established:
  - "Mock-recording backend for volatile seams: Vec arena carved into regions + shared op log + device-side pokes that bypass the log; assert_subsequence() asserts relative order only"
  - "Device-side cursor independence: the mock device tracks its OWN used.idx (read back from the buffer), never the driver's last_used — multiple completions before a drain are the norm"

requirements-completed: [NETDRV-02]   # copied verbatim from plan frontmatter; REQUIREMENTS.md checkbox gated by ready-ids (NETDRV-02 is shared with 07-06 — not marked until the last declaring plan finishes)

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "PCI capability model: Caps::parse yields common/notify/ISR/device triples + notify_off_multiplier from a synthetic cap list; unknown cfg_type skipped per §4.1.4.1; truncated/missing lists error loudly"
    requirement: NETDRV-02
    verification:
      - kind: unit
        ref: "port/src/virtio_tests.rs#caps_parse_yields_the_typed_triples_and_notify_multiplier, #caps_parse_truncated_list_errors_loudly, #caps_parse_missing_list_errors_loudly — via cargo test -p port virtio (exit 0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "§3.1.1 negotiation machine: step order with bounded reset read-back wait; accept mask exactly MAC(5)|STATUS(16)|VERSION_1(32) and nothing else — not even transiently (the driver_feature write log pins it); VERSION_1-absent → loud reject + FAILED; FEATURES_OK not reflected → reject + FAILED"
    requirement: NETDRV-02
    verification:
      - kind: unit
        ref: "port/src/virtio_tests.rs#negotiate_accepts_exactly_mac_status_and_version1, #negotiate_rejects_loudly_without_version1_and_marks_failed, #negotiate_rejects_when_features_ok_is_not_reflected, #negotiate_step_order_is_the_3_1_1_sequence_and_reset_waits_for_readback — exit 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "Split-virtqueue ring index model: descriptor-pool alloc/free by index, 2-desc chain assembly (RX both WRITE / TX neither), avail pre-linking, TX post barrier sandwich (desc → avail entry → fence(Release) → idx → fence(Release) → used flags → notify), notify suppression, acquire-ordered used-drain + slot recovery + V5 id/len faults, u16 wrap at 2^16 over 66000 post/retire cycles, RX replenish-until-caught-up then ONE notify, queue_enable-LAST/DRIVER_OK ordering, bad-qsize rejection"
    requirement: NETDRV-02
    verification:
      - kind: unit
        ref: "port/src/virtio_tests.rs — 12 ring/queue tests (descriptor_pool_…, chain_assembly_…, avail_ring_prelinked_…, tx_post_…×3, used_drain_…×3, rx_replenish_…, indices_wrap_…, ring_rejects_…, queue_setup_…) — exit 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "12-byte virtio_net_hdr model under VERSION_1 (Pitfall N1): const size assertion + zeroed TX form (§5.1.6.2.1) + LE layout pin"
    requirement: NETDRV-02
    verification:
      - kind: unit
        ref: "port/src/virtio_tests.rs#virtio_net_hdr_is_12_bytes_with_a_zeroed_tx_form — exit 0"
        status: pass
    human_judgment: false
  - id: D5
    description: "D-01 provenance record in HowItWorks.md (NETDRV-02's record requirement): from-scratch choice, control/audit rationale, virtio-drivers 2025-09 stale-caveat note, costly reversibility rating, D-02 generic-transport-first-consumer shape; virtio.rs header documents the 07-06 seam + boot call order"
    requirement: NETDRV-02
    verification:
      - kind: other
        ref: "grep -q 'virtio-drivers' ../r9/HowItWorks.md && grep -q 'costly' ../r9/HowItWorks.md → PROVENANCE-RECORDED"
        status: pass
    human_judgment: false
  - id: D6
    description: "Workspace CI legs green with the new module (the shared-crate rule — port compiles for raspi/x86_64/riscv64 unchanged)"
    requirement: NETDRV-02
    verification:
      - kind: other
        ref: "cargo xtask test --verbose && cargo xtask check --verbose && cargo xtask clippy --verbose — exit 0 (twice: task gate + tracer feedback gate re-run)"
        status: pass
    human_judgment: false

# Metrics
duration: 19 min
completed: 2026-11-04
status: complete
---

# Phase 07 Plan 03: From-Scratch Virtio Transport — Negotiation Machine + Split-Ring Index Model, Host-Proven Summary

**port::virtio landed as an unsafe-free (forbid-enforced), from-scratch virtio 1.x modern transport — PCI capability walk, the §3.1.1 negotiation machine pinned to exactly MAC|STATUS|VERSION_1, and the split-virtqueue ring index model with barrier placement proven as operation sequences by 21 crate-root host tests — plus the D-01 provenance record in HowItWorks.md.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-11-04T23:21:30Z
- **Completed:** 2026-11-04T23:40:58Z
- **Tasks:** 2
- **Files modified:** 4 (all r9-side; zero Harvey repo source changes this plan)

## Accomplishments

- Task 1 (tracer, tdd): `port/src/virtio.rs` (778 lines) — `Caps::parse` (§4.1.4 vendor-capability walk: typed triples + notify_off_multiplier, unknown cfg_type skipped, truncated/non-terminating/incomplete lists loud); `negotiate` (the §3.1.1 sequence: bounded reset read-back spin, ACKNOWLEDGE→DRIVER, two-window feature read, the D-03 subset write — exactly MAC(5)|STATUS(16) low + VERSION_1(32) high, MRG_RXBUF/CSUM/EVENT_IDX never accepted even transiently — FEATURES_OK + re-read check; loud rejects leave the device FAILED); queue helpers (`queue_size`/`queue_notify_off`/`set_queue_addresses`/`enable_queue`/`set_driver_ok`) carrying the queue_enable-LAST / DRIVER_OK-before-notify contract; `Ring<M: RingMem>` — the allocator.rs index/arena idiom re-expressed (descriptor index pool, 2-desc chains RX-WRITE/TX-none, avail pre-linking, the §2.7.13 barrier sandwich on post, replenish-until-caught-up then ONE notify, §2.7.14 acquire-ordered drain with slot recovery and V5 id/len validation); the 12-byte `VirtioNetHdr` model with a const size assert (Pitfall N1)
- Task 1 test half: `port/src/virtio_tests.rs` (723 lines) — 21 tests over Vec-backed mock arenas whose every volatile read/write and fence lands in a shared op log; the barrier sandwich and the §3.1.1 step order are asserted as SEQUENCES (`assert_subsequence`), including a 66000-cycle u16-wrap drive
- Workspace CI legs green with the new module, twice (task gate + tracer feedback gate re-run per the end-of-phase automated-only tracer row): `cargo xtask test` (port lib 38/38 incl. the 21), `check` (aarch64/riscv64/x86_64/port all Finished), `clippy` (workspace Finished; only the pre-existing x86_64 warning)
- Task 2: HowItWorks.md's `### virtio` section is the D-01 provenance record (from-scratch choice, control/audit rationale, the virtio-drivers VERSION_1 caveat's 2025-09 upstream resolution noted as stale, costly reversibility rating, D-02's generic shape); virtio.rs's module header documents the 07-06 seam (PciCfg/CommonCfg/RingMem arch-side implementations, the ISR-byte read-to-ack reader) and the boot call order

## Task Commits

Each task was committed atomically (r9 repo, branch main — the dual-repo contract):

1. **Task 1 RED: virtio transport failing behavior tests** — `0d03536` (test) — exit 101, all 21 target tests failed on behavior assertions (`RED_EVIDENCE_OK`; record `07-03-RED-task1.json` in the phase dir)
2. **Task 1 GREEN: virtio transport implementation** — `61bb840` (feat) — 21/21 pass
3. **Task 1 REFACTOR: lib.rs comment drops the literal forbid token** — `2793869` (refactor) — keeps the acceptance grep at exactly +1
4. **Task 2: D-01 provenance record + 07-06 seam contract** — `bf11fe4` (docs)

**Plan metadata:** docs commit rides on top in the Harvey repo (SUMMARY.md + RED record + STATE.md + ROADMAP.md), per the dual-repo contract.

_TDD gate compliance (Task 1, type=tracer tdd="true"):_ RED `0d03536` → GREEN `61bb840` → REFACTOR `2793869` (comment-only). RED evidence: `gsd_run check tdd-red-evidence` returned `RED_EVIDENCE_OK` (target test `virtio_tests::tx_post_barrier_sandwich_is_the_2_7_13_order` failed as expected, 21 failed / 0 passed, no compile errors, no zero-test discovery). `git log` order: RED is GREEN's direct parent.

## Files Created/Modified

- `r9:port/src/virtio.rs` (NEW, 778 lines) — the transport: capability model, negotiation machine, queue helpers, ring index model, header model; `#![forbid(unsafe_code)]` inner attribute + the lib.rs outer forbid
- `r9:port/src/virtio_tests.rs` (NEW, 723 lines) — 21 crate-root tests + the MockMem/MockCommonCfg/SliceCfg recording mocks
- `r9:port/src/lib.rs` (+7 lines) — `#[forbid(unsafe_code)] pub mod virtio;` + `#[cfg(test)] mod virtio_tests;` at the crate root (allocator_tests precedent)
- `r9:HowItWorks.md` (+8 lines) — the `### virtio` provenance section under `## port`

## Decisions Made

1. **Fence-as-trait-method shape.** `RingMem::fence(order)` has a default body calling `core::sync::atomic::fence` (the RESEARCH-verified dmb ish/ishld lowering); production ring code calls `self.avail.fence(Ordering::Release)` at the spec's publication points, and the test mock overrides `fence` to record position — the barrier sandwich is proven as an operation sequence, and the module keeps zero unsafe / zero asm!.
2. **Avail pre-linking made literal.** `Ring::new` writes every avail entry once (each position permanently names its slot's chain head); post re-writes the same fixed value inside the publication sequence. A fresh ascending descriptor pool reproduces ethervirtio.c's 2i chain-head layout organically — slot i's chain is allocated, not computed.
3. **MAX_QSIZE=256 ceiling with loud construction failure** (the plan's EDGE-PROBE assumption made honest): qemu's queues are accepted as-offered and never downsized; a larger offer fails `Ring::new` instead of truncating the fixed side tables.
4. **Odd used-ring ids rejected as device faults** (hardening beyond the plan's named `id < qsize`): chain heads are even, so an odd id would alias the wrong slot — the device is untrusted input (T-07-05/V5).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Ring hardening beyond the plan's named checks**
- **Found during:** Task 1 (GREEN implementation)
- **Issue:** the plan's V5 validation names `id < qsize`, `len ≤ buffer`, `len ≥ 12`. Three adjacent gaps in the same boundary class were unguarded: an odd used-ring id (chain heads are even — aliasing the wrong slot), a queue size beyond the index model's fixed side tables (the flagged "accept 256, never downsize" assumption had no enforcement edge), and direction misuse (`tx_post` on an RX ring).
- **Fix:** `next_used` rejects odd ids as device faults; `Ring::new` rejects qsize 0 / non-power-of-two / > MAX_QSIZE loudly; `tx_post`/`rx_replenish` guard direction with loud errors. Each pinned by its own test.
- **Files modified:** port/src/virtio.rs (ring validation), port/src/virtio_tests.rs (`used_drain_rejects_out_of_range_and_odd_ids`, `ring_rejects_bad_queue_sizes`; direction guards ride the post/replenish paths)
- **Verification:** 21/21 green; the checks are the T-07-05 mitigation's completed shape
- **Committed in:** `61bb840` (part of the GREEN commit)

---

**Total deviations:** 1 auto-fixed (Rule 2). **Impact on plan:** the hardening completes the plan's own threat-register mitigation (T-07-05's "checked results"); zero scope creep — no API surface beyond the plan's shape, no new dependencies (the Cargo diff is empty).

## Issues Encountered

- **RED phase took three iterations to reach a valid RED** (all pre-commit, 07-01's Debug-derive precedent class): (1) `DescPool::new` was private → E0624 compile error (INVALID_RED — fixture crash, not behavior failure) → `pub(crate)`; (2) a stub `Err` from `Caps::parse` satisfied the error-path tests trivially → stub changed to a degenerate `Ok`; (3) a no-op `tx_post` stub satisfied the notify-suppression test trivially (no notify write, assertion vacuously true) → the test now also asserts the post's idx advance. Final RED: exit 101, 21 failed / 0 passed, all on behavior assertions.
- **GREEN surfaced two test-fixture bugs** (implementation was right per ethervirtio.c; the fixtures were wrong): (1) `device_complete` initially drove the used ring from the driver's `last_used` — two completions before a drain overwrote the same entry; fixed to track the DEVICE's own cursor (read back from the buffer), which is also the more faithful device model; (2) the slot-recovery test expected an immediate re-post to reuse the retired slot — the slot is chosen by the avail POSITION (ethervirtio.c's `avail->idx & (qmask>>1)`), so recovery is exercised when the position comes back around (posts 1..3 first). Both fixed test-side; the implementation did not change.
- Pre-existing warnings observed and deliberately untouched (scope boundary): x86_64 clippy `unnecessary_mut_passed`, port lib test warnings (unused imports in `port/tests/`), rustfmt drift in allocator.rs and other untouched files. None red any gate.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **07-06 (virtio-net driver bind):** the seams it implements arch-side are named and contract-pinned — `PciCfg` over the mapped ECAM window, `CommonCfg` over BAR4 (the device-config region and the ISR-byte read-to-ack reader share the shape), `RingMem` over pagealloc pages mapped Normal at KZERO+PA — and the boot call order (caps → negotiate → ring pages → `Ring::new` → queue_size → set_queue_addresses → queue_notify_off → queue_enable LAST → DRIVER_OK → rx_replenish; never notify before DRIVER_OK) is documented in the module header AND pinned by `queue_setup_writes_addresses_then_enable_last_and_driver_ok`. The negotiated-`FeatureMask` print at bind is the audit line the boot log asserts.
- **07-02 (PCI/ECAM discovery):** unaffected; the transport consumes whatever device discovery hands it (the cap walk starts from a `PciCfg`).
- **NETDRV-02 requirement checkbox:** shared with 07-06 (which declares it too) — intentionally NOT marked Complete until 07-06 lands (the ready-ids shared-ID gate).
- No blockers.

## Self-Check: PASSED

- Files: `r9:port/src/virtio.rs` FOUND (778 lines ≥ 200), `r9:port/src/virtio_tests.rs` FOUND (723 lines ≥ 120), `r9:port/src/lib.rs` contains `pub mod virtio` (forbid count 21 = 20+1 at phase start), `r9:HowItWorks.md` contains the virtio section (greppable: `virtio-drivers` + `costly`)
- Commits (r9): `0d03536` FOUND (test/RED), `61bb840` FOUND (feat/GREEN), `2793869` FOUND (refactor), `bf11fe4` FOUND (docs) — `git log --oneline` verified; RED is GREEN's direct parent
- Measured commit count from ledger: `git rev-list --count aaedaa7..bf11fe4` = 4 ✓
- Acceptance criteria re-run at summary time: `grep -c 'forbid(unsafe_code)' virtio.rs` = 1; `fence(Ordering::Release)` = 6, `fence(Ordering::Acquire)` = 2; `0x1041|VERSION_1|MRG_RXBUF` = 13; `cargo test -p port virtio` 21/21 green; `cargo xtask test/check/clippy` exit 0 (final re-run after all commits)
- Threat-register checks: T-07-05 (used-ring id/len checked results) implemented + test-pinned; T-07-SC (zero new dependencies) — `git diff aaedaa7..bf11fe4 --stat` shows 4 files, all additive, no Cargo.toml/Cargo.lock touch

---
*Phase: 07-r9-pci-ecam-virtio-net-devether*
*Completed: 2026-11-04*
