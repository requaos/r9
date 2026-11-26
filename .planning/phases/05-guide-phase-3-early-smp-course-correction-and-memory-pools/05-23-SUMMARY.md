---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 23
subsystem: aarch64
tags: [smp, aarch64, dtb, dts, qemu-virt, gicv2, gicv3, lowecam, psci, cpu-on, smp-04, smp-09]

# Dependency graph
requires:
  - phase: 05-09
    provides: aarch64/src/smp.rs's DTB /cpus + /psci discovery walk and PSCI CPU_ON with PSCI_RET_INVALID_PARAMETERS (-2) tolerated as an absent secondary CPU, which these regenerated blobs feed
provides:
  - aarch64/lib/virt-gicv2.{dts,dtb}: 2-CPU GICv2 QEMU virt pair (cpu@0 reg <0x00>, cpu@1 reg <0x01>, enable-method "psci", core0+core1 cpu-map) for TCG -smp 2 and -smp 1 boots
  - aarch64/lib/virt-gicv3.{dts,dtb}: 2-CPU GICv3 QEMU virt pair for HVF/TCG -smp 2 and -smp 1 boots (GICR window 0xf60000 already covers both CPUs)
  - aarch64/lib/virt-gicv2-lowecam.{dts,dtb}: 2-CPU low-ECAM GICv2 pair with the moved PCI window preserved (reg <0x00 0x3f000000 0x00 0x1000000>, bus-range <0x00 0x0f>)
affects: [05-22 aarch64 -smp 2 boot gate (loads these blobs via -dtb $env(R9_DTB) so the smp.rs /cpus walk discovers cpu@1 for PSCI CPU_ON), Owner Stop 2]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (28000, confidence: low).
actuals:
  tokens: 5609
  tasks: 2
  commits: 2
  plan_head_before: 41a7f8d33cd1283066811451fc52d0903f4d2da2
  plan_head_after: d2afa95ce07f34087cbb8f79931bb5066c672fa0

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "2-CPU static DTB convention: qemu does not inject /cpus/cpu@1 into a user-supplied -dtb blob, so the committed blobs carry both CPU nodes; PSCI CPU_ON INVALID_PARAMETERS (-2) keeps -smp 1 boots clean against a 2-CPU blob (D-24)"
    - "QEMU dumpdtb regeneration recipe: nix develop -c qemu-system-aarch64 -M virt,gic-version={2,3}[,highmem-ecam=off],dumpdtb=... -cpu cortex-a72 -smp 2, dtc decompile, restore header + /chosen seeds to isolate the perturbation, dtc recompile"
    - "Dump-time phandle renumbering is consistent within the blob (intc/v2m/pl061 shift by one when cpu@1 adds a phandle); PPI CPU-mask flags flip 0x104 -> 0x304 on GICv2 timer/pmu when -smp 2"

key-files:
  created: []
  modified:
    - aarch64/lib/virt-gicv2.dts
    - aarch64/lib/virt-gicv2.dtb
    - aarch64/lib/virt-gicv3.dts
    - aarch64/lib/virt-gicv3.dtb
    - aarch64/lib/virt-gicv2-lowecam.dts
    - aarch64/lib/virt-gicv2-lowecam.dtb

key-decisions:
  - "Headers reworded to avoid the literal substring 'cpu@1' so the plan's verify contract (grep -v comments | grep -c 'cpu@1' == 2 across gicv2+gicv3, == 1 for lowecam) counts only the real node declarations"
  - "/chosen rng-seed and kaslr-seed carried verbatim from the previous commits (and from virt-gicv2.dts into lowecam, per the existing convention) so the committed diffs isolate exactly the -smp 2 perturbation instead of dump-time randomness"

patterns-established:
  - "SMP DTB provenance header: names -smp 2 as load-bearing, explains the PSCI INVALID_PARAMETERS absent-CPU path, and enumerates the consistent-but-real dump deltas (phandle renumber, PPI CPU-mask flags) versus the dump-time noise (/chosen seeds)"

requirements-completed: [SMP-04, SMP-09]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "aarch64/lib/virt-gicv2.{dts,dtb} and aarch64/lib/virt-gicv3.{dts,dtb} define both cpu@0 (reg = <0x00>) and cpu@1 (reg = <0x01>) under /cpus with method = \"hvc\" and cpu_on = <0xc4000003>, so QEMU -smp 2 -dtb boots let aarch64/src/smp.rs discover and start CPU 1 via PSCI CPU_ON (SMP-04, D-25)"
    requirement: SMP-04
    verification:
      - kind: automated
        ref: "grep -v '^[[:space:]]*//' aarch64/lib/virt-gicv2.dts aarch64/lib/virt-gicv3.dts | grep -c 'cpu@1' == 2; method = \"hvc\" + cpu_on = <0xc4000003> present in both; dtbs freshly compiled via dtc (8075/8129 bytes)"
        status: pass
    human_judgment: false
  - id: D2
    description: "aarch64/lib/virt-gicv2-lowecam.{dts,dtb} defines both cpu@0 and cpu@1 while preserving its highmem-ecam=off PCI window (reg <0x00 0x3f000000 0x00 0x1000000>, bus-range <0x00 0x0f>), and cargo test -p xtask passes cleanly (SMP-04, SMP-09)"
    requirement: SMP-09
    verification:
      - kind: automated
        ref: "grep -v '^[[:space:]]*//' aarch64/lib/virt-gicv2-lowecam.dts | grep -c 'cpu@1' == 1; nix develop -c cargo test -p xtask rc 0 (233 tests, 0 failed)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Because aarch64/src/smp.rs treats PSCI_CPU_ON INVALID_PARAMETERS (-2) as an absent secondary CPU, these 2-CPU static DTBs boot cleanly under both -smp 2 and R9_SMP=1 (SMP-04, SMP-09, D-24)"
    requirement: SMP-04
    verification:
      - kind: automated
        ref: "aarch64/src/smp.rs:222 PSCI_RET_INVALID_PARAMETERS = -2 path unchanged; enable-method = \"psci\" on all six cpu nodes; full -smp 2/-smp 1 boot proof is Plan 05-22's scope"
        status: pass
    human_judgment: false

# Metrics
duration: 3min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 23: Regenerate aarch64 virt DTBs with -smp 2 Summary

**All three committed aarch64 QEMU virt static device-tree pairs (virt-gicv2, virt-gicv3, virt-gicv2-lowecam) regenerated with `-smp 2` so `/cpus` declares cpu@0 and cpu@1 with `enable-method = "psci"` — unblocking Plan 05-22's `-smp 2` bring-up while PSCI `INVALID_PARAMETERS (-2)` keeps `-smp 1` boots clean (SMP-04, SMP-09, D-24, D-25).**

## Performance

- **Duration:** 3 min
- **Started:** 2026-11-26T07:51:21Z
- **Completed:** 2026-11-26T07:54:18Z
- **Tasks:** 2 of 2 executed
- **Files modified:** 6 (0 created)

## Accomplishments

- **SMP-04/SMP-09 — 2-CPU DTBs committed.** Each pair was dumped from pinned `qemu-system-aarch64` 11.1.2 with `-cpu cortex-a72 -smp 2` (plus `highmem-ecam=off` for the lowecam fixture), decompiled with `dtc` 1.8.1, diff-reviewed against HEAD, had its provenance header updated and its `/chosen` seeds restored, and was recompiled to `.dtb` from the committed `.dts`. `/cpus` now declares `cpu@0` (`reg = <0x00>`) and `cpu@1` (`reg = <0x01>`), both with `enable-method = "psci"`, and `cpu-map` carries `core0` + `core1` in all three files.
- **Consistent dump deltas verified.** The `-smp 2` dump renumbers phandles by one (intc `0x8002 -> 0x8003`, v2m/its `0x8003 -> 0x8004`, pl061 `0x8004 -> 0x8005`) — every reference is updated consistently within each blob. On GICv2 the timer and pmu PPI interrupt flags flip `0x104 -> 0x304` (per-CPU PPI on both cores); on GICv3 those flags are plain `0x04` and unchanged, and the GICR window (`0xf60000`) already covers both CPUs. The lowecam PCI perturbation is preserved exactly (`reg = <0x00 0x3f000000 0x00 0x1000000>`, `bus-range = <0x00 0x0f>`, byte-identical interrupt-map).
- **Dump-time noise excluded.** `/chosen` `rng-seed`/`kaslr-seed` values were carried verbatim from the previous commits (lowecam keeps the virt-gicv2 values per its standing convention), so each committed diff isolates only the `-smp 2` perturbation.
- **xtask DTB tests green.** `nix develop -c cargo test -p xtask` exits 0 — 233 tests across lib, bin, main, threat_model and doctests, 0 failed.

## Task Commits

Each task was committed atomically:

1. **Task 1: Regenerate virt-gicv2.{dts,dtb} and virt-gicv3.{dts,dtb} with -smp 2** — `38c575a` (feat) — dumpdtb + dtc round-trip of both pairs, headers updated, seeds restored, dtbs recompiled
2. **Task 2: Regenerate virt-gicv2-lowecam.{dts,dtb} with -smp 2 and verify xtask DTB tests** — `d2afa95` (feat) — same recipe for the low-ECAM fixture, low-ECAM PCI window confirmed preserved, `cargo test -p xtask` rc 0

## Files Created/Modified

- `aarch64/lib/virt-gicv2.dts` — regenerated with `-smp 2`: `cpu@1` + `core1` added, `enable-method = "psci"` on both CPU nodes, phandles renumbered, PPI flags `0x104 -> 0x304`, provenance header updated (names `-smp 2` load-bearing, explains the PSCI `INVALID_PARAMETERS` absent-CPU path under `-smp 1`)
- `aarch64/lib/virt-gicv2.dtb` — recompiled from the `.dts` (7785 -> 8075 bytes)
- `aarch64/lib/virt-gicv3.dts` — regenerated with `-smp 2`: `cpu@1` + `core1` + `enable-method = "psci"`, phandles renumbered, GICR window already covers both CPUs, timer/pmu flags unchanged
- `aarch64/lib/virt-gicv3.dtb` — recompiled from the `.dts` (7839 -> 8129 bytes)
- `aarch64/lib/virt-gicv2-lowecam.dts` — regenerated with `-smp 2,highmem-ecam=off`: same CPU deltas as virt-gicv2, low-ECAM PCI window preserved
- `aarch64/lib/virt-gicv2-lowecam.dtb` — recompiled from the `.dts` (7785 -> 8075 bytes)

## Decisions Made

- **Headers avoid the literal `cpu@1` substring.** The plan's verify contract counts `cpu@1` occurrences after stripping `//`-style comments, expecting exactly one per file (the node declaration line). The provenance paragraphs describe the second core as "two CPU nodes (reg = <0x00> and reg = <0x01>)" instead, keeping the verify meaningful without weakening the documentation.
- **`/chosen` seeds restored post-dump.** QEMU randomises `rng-seed`/`kaslr-seed` on every dumpdtb; the committed files keep the prior values (and lowecam keeps virt-gicv2's, per its header's standing convention) so diffs show only the machine perturbation. QEMU overrides both at `-dtb` load time anyway, so this is cosmetic-diff hygiene, not behaviour.

## Deviations from Plan

None - plan executed exactly as written. The recipe in the committed headers (dumpdtb → dtc decompile → review diff → restore header/seeds → dtc compile) was followed verbatim with `-smp 2` added, matching the plan's action steps.

## Issues Encountered

- The `dtc` recompile emits pre-existing `clocks_property`/`gpios_property` warnings (phandle cells not recognised without plugin context); the same warnings fire when compiling the HEAD versions, so they are not introduced by this change and no action was taken (scope boundary).
- Initial draft of the updated headers mentioned `cpu@1` literally, which inflated the verify count (8 vs expected 2); the headers were reworded before commit so the verify contract counts only real node declarations. Fixed in-flight before any commit — not a committed deviation.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. The regenerated blobs are complete and functional; the `-smp 2` boot that consumes them is Plan 05-22's scope by design (this plan's must_haves cover the blobs themselves plus the `xtask` test gate).

The broken-windows ledger (`.planning/WINDOWS.md`) has no entries to add: no stubs, no skipped tests, no unrun `<verify>` — both `<automated>` verifies ran and passed, plus the full plan-level verification block.

## Threat Flags

None new. The plan's threat register is implemented as written: T-05-40 (DTB tampering) is mitigated by generating every `.dts` strictly from pinned nixpkgs `qemu-system-aarch64` `dumpdtb` with `-cpu cortex-a72 -smp 2`, inspecting diffs against HEAD (all deltas enumerated above and consistent), and compiling each `.dtb` directly from the committed `.dts` via pinned `dtc` 1.8.1. T-05-SC: `Cargo.lock` is byte-unchanged — zero external packages installed. No new network endpoints, auth paths, file-access patterns, or trust-boundary schema changes.

## Next Phase Readiness

- **Plan 05-22 (aarch64 -smp 2 boot gate):** the committed blobs now let `smp::discover` find CPU 1 under `-smp 2 -dtb $env(R9_DTB)`; 05-22 wires `start_secondaries` into `main9`, replaces the `secondary_main` spin stub, and flips the gate configs to `-smp 2` with one smoke leg kept at `-smp 1` (D-24).
- **Owner Stop 2 (D-32):** the cumulative diff for the stop gains these six files; no `port/src` changes in this plan.

---

*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- All 6 modified files exist on disk (verified with `[ -f ]`): `aarch64/lib/virt-gicv2.{dts,dtb}`, `aarch64/lib/virt-gicv3.{dts,dtb}`, `aarch64/lib/virt-gicv2-lowecam.{dts,dtb}`; the SUMMARY itself exists.
- All 2 plan commits are ancestors of HEAD (verified with `git merge-base --is-ancestor`): `38c575a` (T1), `d2afa95` (T2).
- Content pins verified: `cpu@1 {` node present in all three `.dts` files; `enable-method = "psci"` present; low-ECAM window `0x3f000000` preserved; compiled `.dtb` blobs round-trip through `dtc` with `cpu@1` intact.
- Measured commits from ledger (`gsd-plan-head-before-05-23` = `41a7f8d3`): 2 at completion; `plan_head_after` = `d2afa95c`.
- Working tree before this SUMMARY's commit: only this SUMMARY untracked — zero generated files left untracked (both `target/virt-gicv*-raw.dtb` removed), `Cargo.lock` untouched.
- Plan-level verification re-run at completion: `cpu@1` counts 2 and 1 (gicv2+gicv3, lowecam) as the plan's verification block requires; `nix develop -c cargo test -p xtask` rc 0 (233 tests, 0 failed).
