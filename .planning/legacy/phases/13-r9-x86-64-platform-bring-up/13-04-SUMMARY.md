---
phase: 13-r9-x86-64-platform-bring-up
plan: 04
subsystem: kernel
tags: [x86_64, q35, acpi, rsdp, xsdt, mcfg, madt, ecam, pciexbar, pci-walk, virtio-net, intx-routing, perturbation-gate, boot-test, qemu-drift]

# Dependency graph
requires:
  - phase: 13-r9-x86-64-platform-bring-up (13-02)
    provides: Facts.acpi_root — the loader-mediated RSDT/XSDT address from the mb2 ACPI tags (live: rsdt 0x3ffe2345), consumed here as the walk's preferred root
  - phase: 13-r9-x86-64-platform-bring-up (13-03)
    provides: the ticking masked-IRQ boot underneath discovery (lapic/timer/trap dispatch, sti LAST), and the gate's ordered-marker machinery this plan extends
provides:
  - The ACPI static-table mini-parser (acpi.rs) — RSDP discovery (mb2 hint first, EBDA/ROM scan fallback), checksum-gated XSDT/RSDT walk, MCFG/MADT/HPET parse, host-tested over synthetic fixtures; the x86 analog of port/fdt.rs
  - The discovery half of pci.rs — PCIEXBAR programming from the parsed MCFG via the ONE CF8/CFC helper (the recorded D-07 carve-out) with the old->new audit line, the recursive bridge walk (pcilscan shape), the virtio-net find with verbatim aarch64 reject logic, the q35 INTx->GSI derivation with provenance print + RCBA IRR cross-check, and the publish-once discovery cell for 13-05's bind
  - The moved-window perturbation gate (config_q35lowmcfg.toml + the committed moved-MCFG artifact): the kernel follows a table that names 0xe0000000 while SeaBIOS's register still says 0xb0000001 — X64-03 closed with teeth
  - The live q35 facts 13-05 consumes: ioapic 0xfec00000 gsib 0, all five IntSrcOverrides (0->2, 5->5, 9->9, 10->10, 11->11), lapic 0xfee00000, hpet 0xfed00000, and the NIC route 00:02.0 INTA -> GSI 22 (PIRQ G)
affects: [13-05, 13-06, "Phase 16 HNET (the netdev/device substrate 13-05 binds)"]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
# Same estimateTokens scale (chars/4 over the realized diff), never a harness token count.
actuals:
  tokens: 20833   # chars/4 over git diff 351287f..f607126 in ../r9 (83332 chars, 6 files; plan estimated 62000, confidence low — the overestimate direction)
  tasks: 3
  commits: 2      # MEASURED: git rev-list --count 171ab0d01..HEAD on the harvey repo (the SUMMARY commit + the metadata commit). Production code: 5 commits in the sibling r9 repo (0c568de, 7d5b850, 772db29, cca67bd, f607126) per the dual-repo execution context.
plan_head_before: 171ab0d018fae0c35780402402b5e8ebb8406dea
plan_head_after: 171ab0d018fae0c35780402402b5e8ebb8406dea   # harvey-side pre-docs HEAD (the 13-03 convention: no harvey production commits this plan); all production commits live in ../r9 (351287f..f607126)

# Tech tracking
tech-stack:
  added: []        # zero new crates — the from-scratch discipline holds
  patterns:
    - "PhysMem-trait parsers: firmware-table walks parameterized over a physical-memory reader (identity-map impl kernel-side, Vec image host-side) — the walk stays pure, unsafe-free, and host-testable"
    - "Positional stage anchors in the gate: a stage whose markers arrive in the SAME serial burst as its predecessor's must anchor past the predecessor's last marker LINE, never at satisfaction-time capture length"
    - "Committed firmware-artifact injection as a qemu drift accommodation: -acpetable file=<committed .bin> replaces a broken -global property with data (the committed-DTB precedent)"

key-files:
  created:
    - ../r9/x86_64/src/platform/q35/acpi.rs
    - ../r9/x86_64/src/platform/q35/pci.rs
    - ../r9/x86_64/lib/config_q35lowmcfg.toml
    - ../r9/x86_64/lib/q35-mcfg-moved.bin
  modified:
    - ../r9/x86_64/src/platform/q35/mod.rs
    - ../r9/xtask/src/main.rs

key-decisions:
  - "MADT entries begin at table offset 44 — after lapic_addr (36) comes the flags dword (40) the first synthetic fixtures shared the misreading of; found by dumping SeaBIOS's real 120-byte MADT through the qemu monitor (the fixture-vs-spec agreement trap, pinned by re-keying the fixtures to the spec with the flags dword present)"
  - "ECAM is memory-mapped config space: byte-granular, NO alignment mask. An & 0xFC cargo-culted from CF8's mechanism #1 into ecam_addr silently redirected every sub-dword field read — the device ID at 0x02 re-read the vendor, so the NIC matched neither modern nor transitional and was skipped without a print; the mask lives ONLY in the CF8/CFC helper where the architecture requires it"
  - "The planned `-global q35-pcihost.MCFG=0xe0000000` perturbation mechanism CRASHES qemu 11.1.2 at machine init (any value, including the default: the property stores into PCIExpressHost.base_addr, the same field pcie_host_mmcfg_unmap uses as its is-mapped state, so a pre-realize set trips the memory_region_del_subregion assert on first reset — verified against v11.1.2 source). Accommodation: -acpetable injects the committed moved-MCFG binary; qemu/SeaBIOS ADD it after the built-ins (two MCFGs in the RSDT, the moved one last) and acpi.rs's walk takes the last MCFG — the ACPI override idiom — with the gate asserting exact bases so any ordering drift reds loudly"
  - "The q35 discovery gate stage anchors POSITIONALLY (just past the last `q35: memory` line) via a new last_marker_line_end helper: the kernel prints the whole facts+discovery block in one serial burst, so a satisfaction-time capture-length anchor skips past the discovery lines that arrived in the same socket read and deadlocks the stage"
  - "Bridge secondary/subordinate bus numbers live at config 0x19/0x1A (the plan text's 0x18/0x1C named the primary-bus and IO-limit slots); the live route facts: virtio-net 1af4:1041 at 00:02.0 rev 01 INTA pin 1, 2 BARs, -> GSI 22 (q35 root slot 2, PIRQ G = 16+6)"

patterns-established:
  - "The q35 discovery serial contract: q35: mcfg <base> buses <range>, pciexbar: <old> -> <new>, pci: bridge ... lines, pci: virtio-net 1af4:1041 at ... pin N bars N, pci: inta -> gsi N (provenance) — every later x86_64 net plan's gate asserts this vocabulary"
  - "The x86 INTx route derivation as data: swizzle (slot+intx)%4 per bridge crossing folded to the root-bus slot, PIRQ = (slot+intx)%4+4 -> GSI 16+PIRQ, with the RCBA IRR cross-check where the registers exist — no per-device IRQ constants"

requirements-completed: [X64-03]

# Coverage metadata (#1602) — one entry per shipped deliverable.
coverage:
  - id: D1
    description: "The RSDP->XSDT->MCFG/MADT/HPET mini-parser (acpi.rs): checksum+signature+bounds gates before any pointer, mb2-hint-first/EBDA-scan-fallback discovery, loud named errors, host-testable over synthetic fixtures"
    requirement: X64-03
    verification:
      - kind: unit
        ref: "r9 host tests: platform::q35::acpi::tests::{golden_mcfg_chain_parses_base_and_bus_range, golden_madt_parses_ioapic_and_overrides, bad_table_checksum_is_a_loud_named_error, bad_signature_and_overrunning_length_are_loud, truncated_madt_entry_is_loud_and_never_panics} (all 5 pass; 17/17 crate-wide)"
        status: pass
      - kind: integration
        ref: "live serial (both gates): 'q35: acpi rsdt 0x3ffe2345 via mb2 acpi tag (5 entries)', 'q35: mcfg 0xb0000000 buses 0x0-0xff (seg 0)', 'q35: ioapic 0xfec00000 gsib 0 (id 0)', all five intsrc lines, 'q35: hpet 0xfed00000'"
        status: pass
    human_judgment: false
  - id: D2
    description: "pci.rs discovery: PCIEXBAR programmed from the parsed MCFG via the ONE CF8/CFC helper (the D-07 carve-out, confined and commented) with the old->new audit print, the recursive bridge walk, the verbatim aarch64 reject logic, the q34 INTx->GSI derivation with provenance, and the publish-once cell"
    requirement: X64-03
    verification:
      - kind: unit
        ref: "r9 host tests: pci::tests::{root_route_matches_the_rotation_tables, root_route_clamps_bogus_pins} (the swizzle fold pinned against the standard rotation tables)"
        status: pass
      - kind: e2e
        ref: "cargo xtask boot-test --arch x86-64 --config q35 PASS x3 consecutive: 'pciexbar: 0xb0000001 -> 0xb0000001' (SeaBIOS's value rewritten harmlessly), 'pci: virtio-net 1af4:1041 at 00:02.0 rev 01 pin 1 (inta) bars 2', 'pci: inta -> gsi 22'"
        status: pass
    human_judgment: false
  - id: D3
    description: "mod.rs wiring: acpi::discover inside print_machine_info, the Acpi facts in a publish-once SyncUnsafeCell for 13-05, one loud fact line per ACPI fact + every walk error printed"
    requirement: X64-03
    verification:
      - kind: integration
        ref: "live serial: the full q35: acpi/mcfg/lapic/ioapic/intsrc/hpet block after the memory facts; loud absence lines verified by construction (each Option matched); the acpi.rs dead-code allow removed (every item now consumed)"
        status: pass
    human_judgment: false
  - id: D4
    description: "xtask gate growth: BootStage::Q35PciDiscovery (mcfg/pciexbar/device markers, order-anchored past the last memory line via last_marker_line_end), the burst-anchor unit test, the per-config finish() contract"
    requirement: X64-03
    verification:
      - kind: unit
        ref: "r9 xtask tests: x64_discovery_markers_count_only_past_the_facts, discovery_anchor_is_the_last_memory_line_not_capture_length (86 green total)"
        status: pass
      - kind: e2e
        ref: "the X64 gate's PASS line names the discovery contract; the pre-fix red (markers not observed with the capture-length anchor) proved the stage has teeth"
        status: pass
    human_judgment: false
  - id: D5
    description: "The moved-window perturbation gate: config_q35lowmcfg.toml + the committed q35-mcfg-moved.bin, the per-config expected bases in finish() (mcfg 0xe0000000, pciexbar -> 0xe0000001) with SHARED device+route markers"
    requirement: X64-03
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch x86-64 --config q35lowmcfg && cargo xtask boot-test --arch x86-64 --config q35 — BOTH PASS in one invocation: the kernel parsed the moved table, moved the window (0xb0000001 -> 0xe0000001), and found the device with the SAME route (inta -> gsi 22); grep e0000000 in the lowmcfg log == 1"
        status: pass
    human_judgment: false

# Metrics
duration: 69 min
completed: 2026-11-07
status: complete
---

# Phase 13 Plan 04: ACPI + PCI/ECAM Discovery Summary

**X64-03 closed with teeth: a checksum-gated RSDP->XSDT->MCFG/MADT mini-parser drives a PCIEXBAR-programming, recursively-walking PCI discovery that follows a moved ECAM window (0xb0000001 -> 0xe0000001 under the perturbation) and still finds the virtio-net NIC with its INTx->GSI route — zero hardcoded addresses, after root-causing one qemu-11.1.2 crash that forced the perturbation mechanism onto -acpitable data injection.**

## Performance

- **Duration:** 69 min
- **Started:** 2026-11-07T14:09:01Z
- **Completed:** 2026-11-07T15:18:14Z
- **Tasks:** 3 (Task 1 auto/tdd, Task 2 tracer, Task 3 auto)
- **Files modified:** 6 in ../r9 (4 created, 2 modified) + this plan's harvey docs

## Accomplishments
- The machine is discovered from firmware data end to end: RSDP via the loader-mediated mb2 tag (fallback EBDA/ROM scan), the RSDT gate-checked, and MCFG/MADT/HPET parsed from the live SeaBIOS tables — ioapic 0xfec00000 gsib 0, all five IntSrcOverrides, lapic 0xfee00000, hpet 0xfed00000 on every gate log
- The kernel PROGRAMS the MCH PCIEXBAR register from the parsed base before the first ECAM access (the SeaBIOS hardcode hazard), through the kernel's single CF8/CFC helper — the D-07 carve-out recorded in the module header, the code comment, and the r9 commit message; all other config access rides ECAM
- The recursive bridge walk + verbatim aarch64 reject logic find the NIC wherever q35 puts it (live: 00:02.0 INTA), and the q35 route derivation (swizzle -> root slot -> PIRQ -> GSI) prints its provenance: `inta -> gsi 22 (q35 root slot 2, pirq G: 16+6)`
- The perturbation proof: under q35lowmcfg the MCFG table names 0xe0000000 while SeaBIOS's register says 0xb0000001 — the kernel follows the table, moves the window, and finds the device with the SAME route; the base window re-greens in the same invocation
- Two live-gate bugs root-caused and pinned as host tests: the MADT offset-44 flags dword and the ECAM byte-granularity (no 0xFC mask)

## Task Commits

Each task was committed atomically (r9 repo, `main`), Task 1 in TDD order:

1. **Task 1 RED: the five failing acpi behavior tests against loud stubs** - `0c568de` (test)
2. **Task 1 GREEN: the RSDP->XSDT->MCFG/MADT/HPET mini-parser** - `7d5b850` (feat)
3. **Task 1 REFACTOR: clippy-clean (is_multiple_of, range contains, let-chains)** - `772db29` (refactor)
4. **Task 2: pci.rs discovery + mod.rs wiring + the discovery gate stage — q35 gate green** - `cca67bd` (feat)
5. **Task 3: q35lowmcfg + the moved-MCFG artifact + per-config assertions — both gates green** - `f607126` (feat)

**Plan metadata:** (this harvey docs commit — see below)

## TDD Gate Compliance

Task 1 executed RED -> GREEN -> REFACTOR per the dispatch's TDD-applicable contract:
- **RED:** commit `0c568de` — all five behavior tests failed on their planned assertions against the empty-facts stubs (left: None / right: golden McfgEntry; the expects; the empty error lists); evidence record with the real command/exit/output at `13-04-red-evidence.json` in this directory. Rust's libtest console has no classifier adapter and `workflow.tdd_mode` is false in config — the record is on disk for review rather than machine-classified (the documented Rust gap, tdd.md #4379)
- **GREEN:** commit `7d5b850` — `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu acpi`: 6 passed (5 target + 1 mb2 filter-hit); the two intermediate failures were fixture bugs (below), not parser escapes
- **REFACTOR:** commit `772db29` — the five clippy lints fixed; tests stay green
- Task 2 added its own host tests (root_route rotation tables) alongside the implementation — they pin the swizzle fold the tracer's live gate exercises

## Files Created/Modified
- `../r9/x86_64/src/platform/q35/acpi.rs` - (new) the mini-parser: ParseError, McfgEntry/IoApicEntry/IntSrcOverride/Madt/RootInfo/Acpi, rsdp_root, fetch_table (the sdtchecksum gate), parse_mcfg/parse_madt/parse_hpet, the PhysMem trait + IdentityMem (volatile identity-map reads), discover (mb2-hint-first walk with EBDA/ROM scan fallback); the five behavior tests
- `../r9/x86_64/src/platform/q35/pci.rs` - (new) the discovery half: the ONE CF8/CFC helper (mch_cfg32), ecam_addr/read8/16/32 over the identity map, the q35 INTx->GSI derivation (root_route + PIRQ table + RCBA IRR cross-check), scan_bus (the recursive pcilscan walk), discover_and_print (PCIEXBAR programming + audit line + publish-once DISCOVERY cell); the root_route host tests
- `../r9/x86_64/src/platform/q35/mod.rs` - the ACPI publish-once cell + print_machine_info grown: root provenance, mcfg, lapic/ioapic/intsrc lines, hpet, every walk error printed, then pci::discover_and_print
- `../r9/x86_64/lib/config_q35lowmcfg.toml` - (new) the perturbation profile: q35 base + [qemu] extra injecting the moved MCFG; the header documents the qemu drift and the two-MCFG override semantics
- `../r9/x86_64/lib/q35-mcfg-moved.bin` - (new) the committed 60-byte moved-MCFG artifact (qemu 11.1.2's own table, base -> 0xe0000000, checksum 0x5c)
- `../r9/xtask/src/main.rs` - the Q35PciDiscovery stage + markers + last_marker_line_end, the per-config q35 finish() contract, the X64 PASS message; 3 new unit tests

## Decisions Made
- The five key-decisions above (MADT offset, ECAM granularity, the -global crash + -acpitable accommodation, the positional anchor, the 0x19/0x1A bridge offsets) — all found by live-gate evidence, all pinned by tests or exact-value gate assertions
- The RCBA IRR cross-check is implemented to the ICH9 spec layout (16-bit regs at RCBA+0x3140+2*(slot-25), nibble per INTx) but is LIVE-UNEXERCISED this plan: the NIC sits at root slot 2 and the registers exist only for slots 25-31. It will fire for a device behind the DMI/ICH9 slots; if the decode is wrong there, it prints a loud MISMATCH (advisory, never gate-driving data) — re-verify when a slot-25+ device appears
- The acpi walk takes the LAST MCFG when several ride the RSDT (the -acpitable reality) — documented in the config header; the gate's exact-base assertions make ordering drift a loud red

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The MADT's fixed tail includes a flags dword: entries start at 44, not 40**
- **Found during:** Task 2 (the tracer gate — `q35: acpi madt table error: BadEntry` on SeaBIOS's real table while the synthetic tests passed)
- **Issue:** parse_madt began the entry walk at offset 40; the spec's MADT header tail is lapic_addr (36) + a flags u32 (40), so every live entry read was off by four. The synthetic fixtures encoded the same misreading — the fixture-vs-parser agreement let both pass while the real table failed
- **Fix:** MADT_FIRST_ENTRY = MADT_FLAGS + 4; the fixtures grew the flags dword (value 1, the live PCAT_COMPAT); the real 120-byte table now parses (5 overrides, ioapic, lapic addr). Found by dumping the live MADT through the qemu monitor (`xp`), decoding it by hand, and reading the entry boundaries
- **Files modified:** ../r9/x86_64/src/platform/q35/acpi.rs
- **Verification:** golden_madt test green over the spec-shaped fixture; the live gate prints the full MADT fact set
- **Committed in:** cca67bd

**2. [Rule 1 - Bug] ecam_addr masked offsets with 0xFC — CF8's alignment cargo-culted into ECAM**
- **Found during:** Task 2 (the tracer gate — `pci: no virtio-net device found` while raw probes at the same addresses read 0x8086/0x1234/0x1af4)
- **Issue:** ECAM is memory-mapped config space and needs byte granularity; the mask redirected every sub-dword read (device ID at 0x02 re-read the vendor, pin at 0x3D read the interrupt LINE, secondary bus at 0x19 read the primary) — the NIC's ID then matched neither modern nor transitional, so the walk skipped it SILENTLY (no reject print fires for an ID that matches nothing)
- **Fix:** the mask lives only in mch_cfg32 (where CF8 genuinely requires it); ecam_addr adds the raw byte offset, with the failure mode documented in its comment
- **Files modified:** ../r9/x86_64/src/platform/q35/pci.rs
- **Verification:** the gate finds and prints the device (00:02.0 pin 1 bars 2) and the route
- **Committed in:** cca67bd

**3. [Rule 3 - Blocking] The planned perturbation mechanism crashes qemu 11.1.2 at machine init**
- **Found during:** Task 3 (the first q35lowmcfg gate run captured zero bytes — qemu died pre-SeaBIOS)
- **Issue:** `-global q35-pcihost.MCFG=<any value, including the default>` trips `memory_region_del_subregion: Assertion 'subregion->container == mr'` at first mch reset: the property stores into PCIExpressHost.base_addr, which pcie_host_mmcfg_unmap() also uses as its is-mapped state, so any pre-realize set corrupts the unmap invariant (verified against the v11.1.2 source fetched this session; a bogus property name errors cleanly, proving the property path is real)
- **Fix:** the accommodation is data — `-acpitable file=x86_64/lib/q35-mcfg-moved.bin` (the committed artifact: qemu's own 60-byte guest MCFG with base -> 0xe0000000 and the checksum recomputed). qemu/SeaBIOS ADD the user table after the built-ins (the guest RSDT carries two MCFGs, moved one last — decoded live from the monitor dump) and acpi.rs takes the last MCFG (the ACPI override idiom). The config header documents the drift; a deferred-items entry routes the README note to 13-06 Task 2
- **Files modified:** ../r9/x86_64/lib/config_q35lowmcfg.toml (new), ../r9/x86_64/lib/q35-mcfg-moved.bin (new)
- **Verification:** q35lowmcfg gate green — the kernel parsed 0xe0000000, moved the window (0xb0000001 -> 0xe0000001), found the device with the same route; base gate re-green in the same invocation
- **Committed in:** f607126

**4. [Rule 1 - Bug] The gate's discovery stage anchored at capture length and deadlocked on the boot burst**
- **Found during:** Task 2 (the tracer gate red with the markers present in the log)
- **Issue:** `pci_after = capture.len()` at facts-stage satisfaction skipped past the discovery lines that had already arrived in the same socket read — the kernel prints the whole facts+discovery block in one burst, so the stage's own markers were always BEFORE its anchor
- **Fix:** the new last_marker_line_end helper anchors POSITIONALLY (just past the last `q35: memory` line), with a unit test demonstrating both the deadlock shape and the fix
- **Files modified:** ../r9/xtask/src/main.rs
- **Verification:** the gate green three consecutive runs
- **Committed in:** cca67bd

**5. [Rule 1 - Bug] The plan's bridge-header offsets named the wrong registers**
- **Found during:** Task 2 (writing scan_bus against the PCI spec)
- **Issue:** the plan text said "secondary/subordinate bus at config 0x18/0x1C"; 0x18 is the PRIMARY bus slot and 0x1C the IO base/limit — secondary/subordinate live at 0x19/0x1A
- **Fix:** CFG_SECONDARY = 0x19, CFG_SUBORDINATE = 0x1A
- **Files modified:** ../r9/x86_64/src/platform/q35/pci.rs
- **Verification:** the pcilscan-shaped walk compiles to the spec layout (no bridge rides the default q35 boot, so the live gate exercises the recursion guard only; the offsets are spec-pinned)
- **Committed in:** cca67bd

---

**Total deviations:** 5 auto-fixed (4 Rule 1 bugs, 1 Rule 3 blocker)
**Impact on plan:** All fixes protect the plan's own gates and the X64-03 proof; the -acpitable accommodation preserves the perturbation's SEMANTICS (the table moves, SeaBIOS's register does not) with committed data instead of a crashing qemu flag. No scope creep — the moved-MCFG artifact is the same shape as the committed-DTB precedent.

## Issues Encountered
- Two RED-phase fixture bugs (RSDP checksum byte at offset 8 not 1; the type-0 LAPIC entry is 8 bytes not 12) — both were fixture-spec errors caught by the failing tests before the GREEN commit, fixed in the same pass (the parser itself needed no change for either)
- The tracer gate run needed three root-cause loops (MADT offset, ECAM mask, gate anchor) before green — each closed with a pinned test or an exact-value assertion

## Verification Log

On the final committed tree (r9 `f607126`):
- `cargo xtask boot-test --arch x86-64 --config q35lowmcfg && cargo xtask boot-test --arch x86-64 --config q35` — **both PASS in one invocation**: lowmcfg shows `q35: mcfg 0xe0000000` + `pciexbar: 0xb0000001 -> 0xe0000001` + the device + `inta -> gsi 22`; the base leg shows `0xb0000000` + `-> 0xb0000001` + the identical device/route facts
- The q35 base gate additionally green three consecutive runs across Tasks 2-3 (the tracer verify plus re-runs)
- `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` — 17 passed (5 acpi + 2 pci new); `cargo test -p xtask` — 86 passed (+2)
- `cargo xtask check` + `cargo xtask clippy` — green, zero warnings in any touched q35 file; aarch64 virt boot-test PASS (R9_HARVEY_DIR set for the smoke initrd)
- Acceptance greps: MCFG=40, APIC|MADT=38, checksum=36 in acpi.rs; 1af4=2, pciexbar=3, CF8=8 (confined to mch_cfg32 — the only pio:: call sites in pci.rs), SyncUnsafeCell=2 in pci.rs; q35-pcihost.MCFG==1 in the lowmcfg config (the drift note); lowmcfg=2 in xtask; the only 0xb0000000-class literals in pci.rs are comments
- No new dependencies: the r9 Cargo.tomls untouched across all five commits

## Known Stubs
None — the discovery cell publish IS this plan's designed end state (13-05's bind consumes it without a second walk).

## Threat Flags
None beyond the plan's threat model: T-13-07's gates (signature+length+checksum before any pointer, MADT entry bounds) are host-tested including the corrupt-table paths; T-13-08's no-fallback window is enforced by the perturbation gate itself (a hardcoded base would red the lowmcfg leg); T-13-09's allowlist-shaped reject logic is the aarch64 code verbatim. The one live-unexercised mitigation surface is the RCBA IRR cross-check decode (advisory prints only; see Decisions).

## Authentication Gates
None - no authentication was required.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- 13-05 (IOAPIC/INTx + the virtio bind) inherits everything data-driven: the Acpi facts cell (ioapic base+gsib and every override), the DISCOVERY cell (device 00:02.0, pin 1, gsi 22), the ecam window facts, and the gate vocabulary to extend (the inta->gsi line is already asserted)
- 13-06 should carry the qemu `-global q35-pcihost.MCFG` crash to build/x86_64/README.md (the deferred-items entry routes it); the committed moved-MCFG artifact regenerates trivially if the qemu pin advances
- X64-03 is closed; X64-02's IOAPIC/under-load halves are 13-05's
- No blockers.

## Self-Check: PASSED

- All 6 r9 key-files exist on disk (`[ -f ]` each: FOUND); the r9 working tree is clean (all five commits landed)
- r9 commits verified: 0c568de (RED), 7d5b850 (GREEN), 772db29 (REFACTOR), cca67bd (Task 2), f607126 (Task 3) are ancestors of r9 `main` HEAD (`git merge-base --is-ancestor`)
- TDD gates: test(13-04) 0c568de precedes feat(13-04) 7d5b850 in ancestry; RED evidence at `13-04-red-evidence.json`
- Production code lives in the sanctioned sibling repo /home/req/code/r9 per the dual-repo execution context; the harvey-side commit for this plan is the docs commit carrying this SUMMARY/STATE/ROADMAP/REQUIREMENTS

---
*Phase: 13-r9-x86-64-platform-bring-up*
*Completed: 2026-11-07*
