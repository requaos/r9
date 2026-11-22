---
phase: 07-r9-pci-ecam-virtio-net-devether
plan: 02
subsystem: infra
tags: [r9, fdt, pci, ecam, interrupt-map, virtio-net, devicetree, aarch64, tdd, dtb-perturbation]

# Dependency graph
requires:
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: 07-01 — the netdev stanza on every aarch64 virt boot (the DTB gains the pci node + interrupt-map + an idle virtio-net device to discover) and the harvey:-prefix/R9_HARVEY_DIR channel
provides:
  - port::fdt interrupt-map surface: find_phandle, node_interrupt_cells, interrupt_map_iter (PciIrqMap), interrupt_map_lookup (masked) + in-file decode tests over the committed DTB
  - aarch64::platform::virt::pci — DTB-driven ECAM discovery (window from translated reg, bus-0 walk, virtio-net 1af4:1041 rev>=1, SPI route decode) with the three-contract-line serial print on every virt boot
  - The NETDRV-01 moved-window perturbation proof: committed virt-gicv2-lowecam.{dts,dtb} + config_virtlowecam.toml, boot green under ECAM 0x3f000000 with the identical route
  - xtask boot-test discovery assertions keyed by config name (virt -> 0x4010000000, virtlowecam -> 0x3f000000) on every virt-family profile
affects: [07-03, 07-04, 07-05, 07-06, 07-07, 07-08, phase-8]

# Actuals (#2632) — pairs with the plan's estimate (tokens 70000, confidence: low).
actuals:
  tokens: 9349      # chars/4 over the realized r9 text diff (bf11fe4..230127f)
  tasks: 2
  commits: 4        # MEASURED: git rev-list --count bf11fe4..230127f (r9); the harvey metadata docs commit rides on top

plan_head_before: bf11fe478fe81002283ad794e198e5b3913a0de6   # r9 repo (code commits)
plan_head_after: 230127fe5d32a9b5f2f5360094c9b0e07143afeb    # r9 repo

# Tech tracking
tech-stack:
  added: []   # zero new crate dependencies; the DTB pair is generated with the devshell-pinned qemu/dtc (T-07-SC vacuous)
  patterns:
    - "interrupt-map decode: stride derived from both nodes' declared cell counts (child #address-cells/#interrupt-cells + parent's via find_phandle), bounds-checked cursor, partial tail terminates (T-07-03)"
    - "Loud-absence DTB discovery for PCI: missing node/reg/map/route prints a named line and nothing binds — no hardcoded-window fallback (the timer.rs documented-fallback idiom explicitly NOT followed)"
    - "DTB-perturbation gate shape: a second committed DTB pair (machine flag moves the window) + a second profile + config-name-keyed assertions — constants live in test expectations only"

key-files:
  created:
    - r9:aarch64/src/platform/virt/pci.rs
    - r9:aarch64/lib/virt-gicv2-lowecam.dts
    - r9:aarch64/lib/virt-gicv2-lowecam.dtb
    - r9:aarch64/lib/config_virtlowecam.toml
  modified:
    - r9:port/src/fdt.rs
    - r9:aarch64/src/platform/virt/mod.rs
    - r9:xtask/src/main.rs

key-decisions:
  - "07-02: fdt decode tests live IN-FILE (fdt.rs #[cfg(test)]) not at crate root — no unsafe/alloc fixtures needed; lib.rs untouched so sibling plans extending the module registry never conflict (the plan-documented deviation from the allocator_tests/virtio_tests precedent)"
  - "07-02: the committed lowecam dts carries the base file's /chosen rng-seed/kaslr-seed verbatim (dump-time noise qemu overrides at -dtb load) so the committed diff isolates exactly the pci reg + bus-range perturbation per the acceptance criterion"
  - "07-02: xtask boot-test finish() presence-asserts the pci discovery lines on every aarch64 virt-family config with the expected ECAM base keyed off config NAME — a single constant serving both windows would prove nothing"

patterns-established:
  - "PciIrqMap tuple + masked (bus,dev,fn,pin)->(SPI,flags) lookup as fdt first-class helpers, heapless from_fn-over-cursor style matching property_range_iter"
  - "pci: ecam base / pci: virtio-net ... / pci: inta -> spi ... serial contract — presence-asserted by boot-test, consumed as the bind surface by 07-05/07-06"

requirements-completed: [NETDRV-01]   # copied verbatim from plan frontmatter; REQUIREMENTS.md checkbox gated by ready-ids (NETDRV-01 is shared with 07-06 — not marked until that plan finishes)

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "port::fdt interrupt-map helpers (find_phandle, #interrupt-cells reader, packed-tuple iterator, masked lookup) with in-file decode tests over the committed virt-gicv2.dtb: 16 entries, dev1/INTA -> SPI 4 level-high, swizzle table, mask discipline, loud absence, partial-tail termination"
    requirement: NETDRV-01
    verification:
      - kind: unit
        ref: "port/src/fdt.rs#fdt::tests — 6 tests via cargo test -p port fdt (exit 0; RED exit 101 first, RED_EVIDENCE_OK)"
        status: pass
    human_judgment: false
  - id: D2
    description: "pci.rs DTB-driven discovery prints the three contract lines on every aarch64 virt boot (ECAM window from the translated reg, virtio-net 1af4:1041 at 00:01.0 rev 01, INTA -> SPI 4 / INTID 36 level-high) — zero address/IRQ constants in kernel code"
    requirement: NETDRV-01
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch aarch64 --config virt + log greps (ecam base 0x4010000000 / device / intid 36); same lines present in virtexec/virtexec3(gicv3)/virtmulti/virtnotes/virtpanic logs"
        status: pass
    human_judgment: false
  - id: D3
    description: "NETDRV-01 moved-window perturbation proof: committed virt-gicv2-lowecam.{dts,dtb} + config_virtlowecam.toml boot discovers and routes the NIC identically under ECAM 0x3f000000 — a hardcoded 0x4010000000 would fail loudly; xtask assertions key the expected base off the config name"
    requirement: NETDRV-01
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch aarch64 --config virtlowecam PASS (log: 'pci: ecam base 0x3f000000 size 0x1000000' + same device/route lines); full regression: all seven virt profiles PASS with the assertions live"
        status: pass
    human_judgment: false

# Metrics
duration: 12min
completed: 2026-11-05
status: complete
---

# Phase 07 Plan 02: fdt Interrupt-Map Helpers + DTB-Driven PCI/ECAM Discovery + the Moved-Window Proof Summary

**r9 now discovers the virtio-net NIC entirely from DTB data — ECAM window from the pci node's translated reg, device by a bus-0 ECAM walk (1af4:1041 rev>=1), interrupt route via the new interrupt-map helpers (SPI 4 / INTID 36, level-high) — printed on every virt boot and proven constant-free by the committed highmem-ecam=off pair: the virtlowecam profile boots the moved window (0x3f000000) and discovers/routes the NIC identically, a boot any hardcoded-window driver would fail loudly.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-11-05T00:07:04Z
- **Completed:** 2026-11-05T00:18:44Z
- **Tasks:** 2
- **Files modified:** 7 (all r9-side; zero Harvey repo source changes)

## Accomplishments

- `port::fdt` gained the four interrupt-map helpers as heapless iterator-shaped extensions (`find_phandle`, `node_interrupt_cells` — loud None when absent, `interrupt_map_iter` with stride derived from both nodes' declared cell counts, `interrupt_map_lookup` applying `interrupt-map-mask` to key and entries) plus `PciIrqMap`; the module keeps `#[forbid(unsafe_code)]` and gained zero unsafe
- In-file `#[cfg(test)]` decode suite over the committed `virt-gicv2.dtb` (6 tests): all 16 tuples decode, the NIC's dev1/INTA route resolves to SPI 4 with flags 0x04, the (dev+pin) swizzle table matches, mask-vs-exact-compare discipline pinned, absent phandle/cells/map are loud None, and a partial tuple tail terminates without panic (T-07-03)
- New `aarch64/src/platform/virt/pci.rs`: DTB-driven discovery in the pl011 idiom — window from `property_translated_reg_iter`, bus-0 1 MiB span mapped via `deviceutil::map_device_register` (no full-window map), walk bounds-checked against the span (T-07-04), transitional 0x1000 and rev-0 devices rejected loudly per D-03; the three contract lines ride `print_machine_info` on every aarch64 virt boot
- NETDRV-01's perturbation criterion proven: committed `virt-gicv2-lowecam.{dts,dtb}` (diff vs base confined to the provenance header + pci `reg` 0x4010000000/256 MiB -> 0x3f000000/16 MiB + `bus-range` 0x00-0xff -> 0x00-0x0f; interrupt-map byte-identical) + `config_virtlowecam.toml`; `boot-test --config virtlowecam` green with the same device/route
- xtask `finish()` now presence-asserts the discovery lines on every aarch64 virt-family config, ECAM base keyed off config name (virt -> 0x4010000000, virtlowecam -> 0x3f000000) — constants live in test expectations only; all seven profiles green

## Task Commits

Each task was committed atomically (r9 repo, branch main — the dual-repo contract):

1. **Task 1 RED: fdt interrupt-map helpers failing behavior tests** — `8fed352` (test)
2. **Task 1 GREEN: fdt interrupt-map helpers** — `e4d120d` (feat)
3. **Task 1: DTB-driven PCI/ECAM discovery + serial prints on virt (pci.rs + mod.rs)** — `951521e` (feat)
4. **Task 2: virtlowecam moved-window profile + boot-test discovery assertions** — `230127f` (feat)

**Plan metadata:** docs commit rides on top in the Harvey repo (SUMMARY.md + STATE.md + ROADMAP.md + the RED evidence record), per the dual-repo contract.

_TDD gate compliance (Task 1, tdd="true"):_ RED `8fed352` (exit 101, five behavior tests failed on assertions — `RED_EVIDENCE_OK` per `gsd_run check tdd-red-evidence`, record at `07-02-RED-task1.json` in this phase dir) → GREEN `e4d120d` (6/6 pass) → REFACTOR not needed (the GREEN bodies were the minimal implementations; nothing to clean). pci.rs/mod.rs rode their own feat commit `951521e` (no host test drives the MMIO walk; verified by the boot gates). Tracer feedback gate: interactive + end-of-phase + automated-only verify → re-ran end-to-end post-commit (`cargo test -p port fdt` 6/6; seven boot profiles green) — verified, expanded.

## Files Created/Modified

- `r9:port/src/fdt.rs` (+269 net) — the four helpers + `PciIrqMap` + `decode_interrupt_map` (bounds-checked cursor) + the in-file test module with the placement-deviation header comment
- `r9:aarch64/src/platform/virt/pci.rs` (NEW, ~160 lines) — `discover_and_print(dt) -> Option<VirtioNetDiscovery>`; loud named prints on every absence path
- `r9:aarch64/src/platform/virt/mod.rs` — `pub mod pci;` + `print_machine_info` invokes discovery; doc line extended ("PCI included")
- `r9:aarch64/lib/virt-gicv2-lowecam.dts` / `.dtb` (NEW) — the highmem-ecam=off committed pair with provenance header
- `r9:aarch64/lib/config_virtlowecam.toml` (NEW) — machine `virt,gic-version=2,highmem-ecam=off` + the lowecam dtb; header names the NETDRV-01 gate purpose
- `r9:xtask/src/main.rs` (+31) — four marker constants + the config-name-keyed discovery assertion block in `finish()`

## Decisions Made

1. **In-file test placement (plan-sanctioned deviation from the crate-root precedent).** The decode tests need no unsafe arena and no mock machinery — only a `std::fs` fixture read — so they live in `fdt.rs` under `#[cfg(test)]`, keeping `lib.rs` untouched for sibling plans' module-registry edits. Documented in the test module header.
2. **`/chosen` seeds carried verbatim into the committed lowecam dts.** qemu re-rolls `rng-seed`/`kaslr-seed` per dump (and overrides them again at `-dtb` load), so the raw dump differed from the base file by seed noise; carrying the base values keeps the committed diff confined to exactly the perturbation (header + pci reg + bus-range), as the acceptance criterion requires.
3. **Stride derivation resolves the parent via the FIRST tuple's phandle.** All 16 committed entries share the GIC parent; an unresolvable/absent parent or a `parent #interrupt-cells > 3` yields the empty iterator (loud None at the lookup), never an assumed stride.
4. **Mask mid/lo cells must be zero.** `PciIrqMap` keys on phys.hi + pin; a mask routing on mid/lo is unsupported-loud (`None`) rather than silently approximated. The committed mask `0x1800/0/0/0x07` satisfies this.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Plan's boot-test verify commands omit `--arch aarch64`**
- **Found during:** Task 1 verification (first boot-test invocation)
- **Issue:** `Arch::from` defaults to x86_64, so a bare `--config virt` looks for `x86_64/lib/config_virt.toml` — the same plan-command gap 07-01 recorded.
- **Fix:** ran all boot-test invocations as `cargo xtask boot-test --arch aarch64 --config <p>` — the plan's obvious intent (every fails_when clause describes the aarch64 profiles).
- **Files modified:** none
- **Verification:** seven profiles green
- **Committed in:** n/a

---

**Total deviations:** 1 auto-fixed (plan-command correction, the 07-01 class). **Impact on plan:** zero design change; the plan's truths all hold as measured.

## Issues Encountered

- `loud_absence_returns_none_never_a_default` passes against the RED stubs by construction (its pins are absence-shaped) — the behavior-5 malformed-tail half lives in `decode_terminates_on_a_partial_tail`, which failed RED as required; recorded in the RED evidence JSON.
- The word "unsafe" appears once in the new test-module header comment (documenting *why* in-file placement is safe) — fdt.rs gained zero unsafe code; the `forbid(unsafe_code)` count (1, a pre-existing comment reference) is unchanged and the attribute holds.
- Pre-existing observed, not touched (scope boundary): 22 aarch64 dead-code warnings on the kernel build; the untracked `07-01-RED-task2.json` evidence file from the prior plan (its commit rides 07-01's history, not this plan's).
- NETDRV-01 is shared with 07-06 per the ready-ids rule — the REQUIREMENTS.md checkbox stays unchecked until 07-06's SUMMARY exists.

## User Setup Required

None - no external service configuration required. (R9_HARVEY_DIR is the established cross-repo channel since 05-10; the verify commands set it explicitly.)

## Next Phase Readiness

- **07-05/07-06 (driver bind + IRQ plumbing):** the serial contract (`pci: ecam base …`, `pci: virtio-net 1af4:1041 at 00:01.0 …`, `pci: inta -> spi N intid M …`) is asserted on every virt profile; `discover_and_print` returns `VirtioNetDiscovery { ecam_base, ecam_size, bus, dev, fun, rev, spi, flags }` — the bind path's input facts. The bus-0 ECAM mapping idiom and the interrupt-map lookup are the reusable pieces.
- **07-03 (virtio transport):** unaffected — port/src/lib.rs untouched by this plan.
- The `virtlowecam` profile is now a permanent regression leg: any future hardcoded-window regression reds it immediately.
- No blockers.

## Self-Check: PASSED

- Files: `r9:port/src/fdt.rs` helpers present (`grep -c find_phandle` = 3, `interrupt_map` = 18); `r9:aarch64/src/platform/virt/pci.rs` FOUND (~160 lines); `r9:aarch64/lib/virt-gicv2-lowecam.{dts,dtb}` + `config_virtlowecam.toml` FOUND and git-tracked (ls-files lists all three); `highmem-ecam=off` count in the config == 1
- Constants audit: `grep -rn '0x4010000000' aarch64/src/ port/src/` → none; `grep -n '\b36\b' pci.rs` → none (the route computes INTID = 32 + SPI)
- Commits (r9): `8fed352`, `e4d120d`, `951521e`, `230127f` all FOUND in `git log`; measured count `git rev-list --count bf11fe4..230127f` = 4 ✓
- Acceptance re-run at summary time: `cargo test -p port fdt` 6/6 pass (exit 0); boot-test-virt.log greps (ecam base 0x4010000000 / virtio-net 1af4:1041 / intid 36) all match; virtexec3 (gicv3) log carries the discovery lines; `boot-test --config virtlowecam` PASS with `pci: ecam base 0x3f000000 size 0x1000000`; all seven profiles PASS with the finish()-time discovery assertions live; dts diff vs virt-gicv2.dts confined to header + pci reg + bus-range (interrupt-map untouched)

---
*Phase: 07-r9-pci-ecam-virtio-net-devether*
*Completed: 2026-11-05*
