---
phase: 03-r9-qemu-virt-platform-bring-up
plan: "04"
subsystem: kernel
tags: [rust, aarch64, r9, qemu-virt, dtb, pl011, gicv2, identity-map, cpacr, tracer, boot-proof]

requires:
  - phase: 03-r9-qemu-virt-platform-bring-up (plan 01)
    provides: "../r9 flake devshell — every build/clippy/qemu invocation ran through `nix develop -c`"
  - phase: 03-r9-qemu-virt-platform-bring-up (plan 02)
    provides: "cfg-gated platform tree (the virt arm compiled out until this plan created platform/virt/), not(virt) raspi couplings, trap init/unmask split"
  - phase: 03-r9-qemu-virt-platform-bring-up (plan 03)
    provides: "xtask [qemu]/[dist] schema + virt qemu arm + override chain — config_virt.toml is consumed as pure data with zero xtask edits"
provides:
  - "Boot-proven virt platform slice: qemu -M virt,gic-version=2 -cpu cortex-a72 -smp 1 -m 1024M -accel tcg boots the cargo-built ELF to the r9 banner (ROADMAP SC#2) — captured serial log shows banner + 'DTB found at: 0xffff800040000000' + zero panic/Unhandled-interrupt markers"
  - "../r9/aarch64/lib/virt-gicv2.{dts,dtb} — committed DTB pair (7785-byte trimmed blob) with the qemu-11.1.2 dumpdtb + dtc regeneration procedure and the -cpu cortex-a72 caveat documented in the .dts header (P11)"
  - "../r9/aarch64/lib/config_virt.toml — the virt platform as pure data (load-address 0xffff800040080000, machine/cpu/smp/memory/accel/dtb, dist format elf)"
  - "P1 fix at BOTH pre-banner DTB consumers: init_vm (pre-MMU, raw PA) and main9 (VA side) substitute x0==0 → fixed PA 0x40000000 under cfg(platform = \"virt\"); the init_vm substitution carries a putstr marker visible in the serial log"
  - "P3/P4 memory-map fixes: per-platform base_physaddr() (virt = boottext_pa, raspi keeps 0) in kmem.rs + vminit.rs; second 1 GiB identity block at PA 0x40000000 on virt"
  - "D-02 early console: init_early_uart/init_early_uart_putc cfg-selected bodies — raspi verbatim, virt zero-init PL011 at PA 0x09000000 (qemu pre-configures it, research Pattern 3)"
  - "platform/virt/{mod,pl011,devcons}.rs — DTB-discovered polled-TX Pl011Uart installed via Console::set_uart (riscv64 virt idiom); print_machine_info(dt) (D-03/D-04)"
  - "Two boot-critical fixes the tracer caught: CPACR_EL1.FPEN enable on direct-EL1 ELF entry; kernel text maps with Page4K on virt (boottext_pa not 2 MiB aligned); vmdebug User-table walk gated to raspi"
affects: [03-05, 03-06, phase-04-userspace-exec, phase-05-int-04]

# Actuals (#2632) — same scale as the plan's estimateTokens (chars/4 over realized diff).
# Cross-repo plan: commits = 2 r9 code commits + 1 harvey docs commit.
# r9 leg: 11196de -> c74fcb1 (diff 28,404 chars -> 7,101 tokens). Harvey leg measured from ledger.
actuals:
  tokens: 7101
  tasks: 2
  commits: 3

# Commit ledger (#3968) — cross-repo plan: harvey base from the persisted ledger;
# the r9 leg is 11196de..c74fcb1 (2 commits; Task 1 was amended once before any
# push — final hash 04fcf12). commits: 3 = 2 r9 + this harvey docs commit.
plan_head_before: 706bd873d53ee57c8822902f3574ce8d1740e98a
plan_head_after: 26c3eea25

tech-stack:
  added: []
  patterns:
    - "x0==0 DTB fallback lives Rust-side at BOTH consumers (init_vm pre-MMU raw-PA, main9 VA-side twin) — asm stays untouched per PATTERNS.md's preferred route; the init_vm putstr marker makes the fallback's firing observable in the serial log"
    - "Per-platform base_physaddr() via cfg blocks — virt starts kernel ranges at the linker's boottext_pa (P3), raspi deliberately keeps 0"
    - "Direct-EL1 entry contract: qemu ELF -kernel skips l.S's el2 path, so EL1-resident enables l.S used to do there (CPACR_EL1.FPEN) must be re-done Rust-side on virt"
    - "dumpdtb provenance: dump with the BOOT cpu (-cpu cortex-a72), not the machine default — the a15 default dump loses 'arm,armv8-timer' and SMC64 psci IDs"

key-files:
  created:
    - "../r9/aarch64/lib/config_virt.toml"
    - "../r9/aarch64/lib/virt-gicv2.dts"
    - "../r9/aarch64/lib/virt-gicv2.dtb"
    - "../r9/aarch64/src/platform/virt/mod.rs"
    - "../r9/aarch64/src/platform/virt/pl011.rs"
    - "../r9/aarch64/src/platform/virt/devcons.rs"
  modified:
    - "../r9/aarch64/src/main.rs"
    - "../r9/aarch64/src/kmem.rs"
    - "../r9/aarch64/src/pre_mmu/vminit.rs"
    - "../r9/aarch64/src/pre_mmu/util.rs"
    - "../r9/aarch64/src/l.S (one-line call-site rename only)"

key-decisions:
  - "CPACR_EL1.FPEN=TrapNothing set Rust-side at the top of init_vm under cfg(virt) — qemu's ELF boot enters DIRECTLY at EL1 (research-verified), skipping l.S's el2 path where FPEN was set, so the first compiler-emitted SIMD instruction (fmov/cnt in pagealloc::usage_bytes) trapped with ESR EC=0x07; raspi is gated out so its register state stays byte-identical"
  - "Kernel text maps with Page4K on virt (raspi keeps Page2M) — boottext_pa=0x40080000 is not 2 MiB aligned and map_phys_range requires page-size-aligned range ends; without this the plan's own P3 fix panics init_vm"
  - "vmdebug::print_recursive_tables(User) gated to raspi — user page tables are never initialized on virt (03-02 gated init_user_page_tables), so the unconditional user walk took a translation fault at FAR 0x0000_ffff_ffff_f000 (the user recursive root) after the banner"
  - "The committed DTB is dumped with -cpu cortex-a72 (the boot cpu) — qemu's default-cpu (a15) dump reports the timer as only arm,armv7-timer and psci with 32-bit IDs; the .dts header records this and the full regeneration procedure (P11)"

patterns-established:
  - "Tracer discipline paid off as designed: three architectural dead-ends (FPEN, 2 MiB alignment, user-table walk) were caught on the boot-proven slice before any GIC/timer/harness investment"
  - "Verify-harness note for 03-05/03-06: 'mkdir -p ... && qemu ... &' binds & to the whole list ($QP names a subshell), and a stale serial log makes the first poll read pre-boot content — the gate is deterministic from a fresh log; the D-11 socket chardev design avoids the pattern (recorded in WINDOWS.md)"

requirements-completed: [R9-01]
# R9-01 is shared by plans 03-02…03-06; the shared-ID gate keeps it open until the
# last declaring plan completes. This plan's copy of the ID is verbatim from the
# plan frontmatter; REQUIREMENTS.md stays Pending until 03-06 finishes.

coverage:
  - id: D1
    description: "Tracer slice: ELF entry → early page tables + pre-MMU DTB parse (init_vm x0==0 → 0x40000000 fallback, putstr marker logged) → main9 (VA-side fallback twin) → pagealloc → Console over DTB-discovered PL011 → banner on the canonical SC#2 machine; kernel at PA 0x40080000 covered by identity block entries[1] AND the higher-half map; GIC/PL011 MMIO never Normal kernel text; raspi builds green; kernel.ld untouched"
    requirement: R9-01
    verification:
      - kind: integration
        ref: "cd ../r9 && nix develop -c cargo xtask build --arch aarch64 --config virt && qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -smp 1 -m 1024M -accel tcg -display none -serial file:target/boot-logs/virt-tracer.log -kernel target/aarch64-unknown-none-elf/debug/aarch64 -dtb aarch64/lib/virt-gicv2.dtb → log greps: banner=1, 'DTB found at: 0xffff800040000000'=1, 'vminit:init_vm: x0==0 … fixed DTB PA 0x40000000' present, panic|Unhandled-interrupt count=0, 'looping now'=1"
        status: pass
      - kind: integration
        ref: "raspi regression: nix develop -c cargo xtask build --arch aarch64 && … --config raspi4b → both Finished, zero errors"
        status: pass
      - kind: other
        ref: "static gate: dts carries arm,cortex-a15-gic + arm,pl011 + regeneration header; dtb non-empty (7785 bytes, decompiles byte-identical to the -cpu cortex-a72 dump); config_virt.toml pure data (platform/load-address/machine/dist greps); vminit.rs carries the 0x4000_0000 substitution; git diff on kernel.ld empty"
        status: pass
    human_judgment: false
  - id: D2
    description: "virt print_machine_info (D-03/D-04): DTB-derived Memory line anchored at 0x40000000 (qemu injects the -m 1024M size at -dtb load) and found: lines for arm,pl011 / arm,cortex-a15-gic / arm,armv8-timer in the print_board_info slot; no hardcoded virt address tables; clippy green on virt"
    requirement: R9-01
    verification:
      - kind: integration
        ref: "boot to target/boot-logs/virt-info.log → 'Machine information:' block shows Memory: 0x40000000 .. +0x40000000, found: arm,pl011, found: arm,cortex-a15-gic, found: arm,armv8-timer; zero wedge markers"
        status: pass
      - kind: other
        ref: "nix develop -c cargo xtask clippy --arch aarch64 --config virt → exit 0; zero warnings in files this plan touched (remaining warnings all pre-existing in untouched files)"
        status: pass
    human_judgment: false

# Metrics
duration: 26 min
completed: 2026-11-01
status: complete
---

# Phase 03 Plan 04: virt platform tracer — boots to the r9 banner Summary

**The phase tracer slice is boot-proven end-to-end: r9's aarch64 crate now boots on the canonical SC#2 machine (`qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -smp 1 -m 1024M -accel tcg`) from the cargo-built ELF with the committed `virt-gicv2.dtb` at PA 0x40000000 — the serial log shows the "r9 from the Internet" banner, the "DTB found at: 0xffff800040000000" line proving the x0=0 fixed-PA fallback at both consumers, the DTB-derived machine-info block (Memory 0x40000000 +1 GiB, PL011/GICv2/timer discovered), and zero panic/Unhandled-interrupt markers, while raspi3b/4b builds and virt clippy stay green and kernel.ld is untouched.**

## Performance

- **Duration:** 26 min
- **Started:** 2026-11-01T23:17:54Z
- **Completed:** 2026-11-01T23:43:52Z
- **Tasks:** 2
- **Files modified:** 12 (all in sibling repo `../r9`; harvey side carries only this SUMMARY + state files per D-13)

## Accomplishments

- **ROADMAP Phase 3 SC#2 satisfied**: the r9 kernel banner prints on the captured serial log of the exact canonical invocation, ELF `-kernel` + `-dtb` — the boot-proven slice every later plan in this phase expands onto.
- **Committed DTB artifacts (P1/P11)**: `aarch64/lib/virt-gicv2.{dts,dtb}` — dumped from qemu-system-aarch64 11.1.2 with `-M virt,gic-version=2,dumpdtb=… -cpu cortex-a72` (the cpu argument is load-bearing: the default a15 dump lacks `arm,armv8-timer` and SMC64 psci IDs), dtc round-tripped to 7785 bytes, both committed mirroring the bcm2710 convention; the .dts header documents the exact regeneration procedure and the /memory + /chosen load-time injection note.
- **P1 fixed at both pre-banner consumers**: `init_vm` (pre-MMU, raw PA — the FIRST DTB consumer) and `main9` (VA side) substitute `x0==0` → fixed PA 0x40000000 under `cfg(platform = "virt")`, with a putstr marker proving the firing in the log; raspi keeps the incoming pointer verbatim. Asm untouched (only the `init_early_uart` rename in l.S, as permitted).
- **P3/P4 memory-map fixes**: `base_physaddr()` parameterized per platform in `kmem.rs` + `vminit.rs` (virt = linker `boottext_pa` — the serial dump shows `boottext: 0x40080000..` and `total: 0x40080000..`, so GIC 0x08000000 / PL011 0x09000000 are never Normal-cacheable kernel text; raspi keeps base 0); a second 1 GiB identity block at PA 0x40000000 covers the virt kernel and DTB for the MMU-on jump.
- **D-02 early console**: `init_early_uart`/`init_early_uart_putc` with cfg-selected bodies — raspi bodies verbatim; virt is zero-init PL011 FR-TXFF poll + DR write at fixed PA 0x09000000 (qemu pre-configures the PL011 — research Pattern 3 confirmed live: the pre-MMU vminit markers print).
- **platform/virt skeleton**: `pl011.rs` (DTB-discovered via `find_compatible("arm,pl011")` → translated reg → `map_device_register`, polled-TX `Uart` impl, no GPIO/mailbox init), `devcons.rs` (riscv64-virt `Console::set_uart` + `SyncUnsafeCell<MaybeUninit>` idiom with the error-tolerant match), `mod.rs` (plus Task 2's `print_machine_info`).
- **D-03/D-04 machine info**: `print_machine_info(dt)` prints `Memory: 0x40000000 .. +0x40000000` (qemu injected the `-m 1024M` size at load) and `found:` lines for `arm,pl011`, `arm,cortex-a15-gic`, `arm,armv8-timer` from the main9 board-info slot — discovery flows through `port::fdt`, no hardcoded address tables.

## Task Commits

Each task was committed atomically (repo: `../r9`, branch `main`):

1. **Task 1 (tracer): DTB artifacts + config_virt.toml + memory-map fixes (P1/P3/P4) + early PL011 (D-02) + platform/virt skeleton + boot** — `04fcf12` (feat) — 11 files, +694/-7
2. **Task 2: print_machine_info (D-03/D-04) + main.rs call site + regression sweep** — `c74fcb1` (feat) — 2 files, +30

**Plan metadata:** harvey docs commit (see below; `docs(03-04): complete virt platform tracer plan`)

_Note: Task 1's commit was amended once before any push to fold in a clippy-warning fix in its own new code (deviation 4); `04fcf12` is the final hash._

## Files Created/Modified

- `../r9/aarch64/lib/virt-gicv2.dts` (created) — human-reviewable DTB source + regeneration-procedure header (qemu 11.1.2, dumpdtb with `-cpu cortex-a72`, dtc round-trip, /memory+/chosen injection note)
- `../r9/aarch64/lib/virt-gicv2.dtb` (created) — 7785-byte compiled blob qemu loads at 0x40000000 via `-dtb`
- `../r9/aarch64/lib/config_virt.toml` (created) — virt platform as pure data: platform="virt", load-address 0xffff800040080000, machine/cpu/smp/memory/accel/dtb, dist format="elf"
- `../r9/aarch64/src/platform/virt/mod.rs` (created) — `pub mod devcons; pub mod pl011;` + `print_machine_info(dt)`
- `../r9/aarch64/src/platform/virt/pl011.rs` (created) — `Pl011Uart` DTB-discovered polled-TX driver (no raspi GPIO/mailbox init)
- `../r9/aarch64/src/platform/virt/devcons.rs` (created) — `Console::set_uart` installation of the PL011
- `../r9/aarch64/src/pre_mmu/vminit.rs` (modified) — per-platform `base_physaddr()`, init_vm x0==0→0x40000000 fallback with putstr marker, CPACR_EL1.FPEN enable (virt), Page4K text map (virt), second identity block at 0x40000000 (virt)
- `../r9/aarch64/src/kmem.rs` (modified) — per-platform `base_physaddr()` (virt = boottext_pa)
- `../r9/aarch64/src/pre_mmu/util.rs` (modified) — `init_early_uart`/`init_early_uart_putc` cfg-selected raspi/virt bodies
- `../r9/aarch64/src/l.S` (modified) — one-line call-site rename only (no x27/x0 fixup)
- `../r9/aarch64/src/main.rs` (modified) — main9 VA-side DTB fallback twin; `print_machine_info` call in the board-info slot; vmdebug User-table walk gated to raspi

(Harvey repo: only this SUMMARY + state files, per D-13.)

## Decisions Made

- **CPACR_EL1.FPEN enabled Rust-side on virt** — qemu's ELF `-kernel` enters directly at EL1 (research-verified), skipping l.S's `el2` path where `CPACR_EL1_FPEN` is set; the first compiler-emitted SIMD instruction (`fmov`/`cnt` in `pagealloc::usage_bytes`) trapped with ESR EC=0x07. Set at the top of `init_vm` (the earliest Rust point) under `cfg(virt)`; raspi is gated out so its register state stays byte-identical. (Deviation 1.)
- **Kernel text maps with Page4K on virt** — `boottext_pa = 0x40080000` is not 2 MiB aligned and `map_phys_range` requires page-size-aligned range ends; the plan's P3 fix (`base_physaddr() = boottext_pa`) would otherwise panic init_vm on the "Kernel Text" map. Raspi keeps Page2M (its ranges are 2 MiB aligned). (Deviation 2.)
- **vmdebug User-table walk gated to raspi** — user page tables are never initialized on virt (03-02 gated `init_user_page_tables` behind the user-page-table experiment), so the unconditional `print_recursive_tables(User)` dereferenced the unmapped user recursive root (translation fault, FAR `0x0000_ffff_ffff_f000`) after the banner. Gated with a Phase-4-revisits comment; the Kernel walk still prints on virt (it proved the maps). (Deviation 3.)
- **DTB dumped with the boot cpu** — `-cpu cortex-a72` is part of the recorded regeneration procedure because qemu's default-cpu dump reports the timer as only `arm,armv7-timer` and 32-bit psci IDs.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] CPACR_EL1.FPEN never set on virt's direct-EL1 ELF entry**
- **Found during:** Task 1 step (7), first boot attempt — banner + DTB line printed, then a TrapFrame wedge at "Memory usage:" (`ec: FloatSimd`, ELR in `usage_bytes`; faulting instruction `fmov s0, w8` — a compiler-emitted NEON `count_ones`)
- **Issue:** qemu's ELF `-kernel` enters at EL1 directly (research-verified), so l.S's `el2:` path — the only place `CPACR_EL1_FPEN` is written — never runs on virt; with FPEN at its reset value, any SIMD/FP instruction traps
- **Fix:** `CPACR_EL1.write(CPACR_EL1::FPEN::TrapNothing)` + `isb` at the top of `init_vm` (earliest Rust point), gated `cfg(all(not(test), platform = "virt"))`; raspi gated out (l.S already writes the same value on its path)
- **Files modified:** ../r9/aarch64/src/pre_mmu/vminit.rs
- **Verification:** disassembly shows `msr CPACR_EL1` at init_vm+0x20; boot proceeds past `usage_bytes`; raspi builds green with unchanged code paths
- **Committed in:** `04fcf12` (Task 1 commit)

**2. [Rule 3 - Blocking] Kernel-text map page size incompatible with the P3 fix on virt**
- **Found during:** Task 1 step (3) implementation (static reasoning, confirmed against `map_phys_range`'s alignment check)
- **Issue:** with `base_physaddr() = boottext_pa = 0x40080000` (the plan's P3 fix), the "Kernel Text" map range starts at a non-2 MiB-aligned address; `map_phys_range` requires page-size-aligned range ends, so the pre-existing Page2M mapping would fail and panic init_vm before any banner
- **Fix:** cfg-select the text map's page size — Page4K on virt, Page2M on raspi (raspi's ranges are 2 MiB-aligned at both ends by the linker script's ALIGNs, unchanged)
- **Files modified:** ../r9/aarch64/src/pre_mmu/vminit.rs
- **Verification:** boot log shows `Kernel Text 0x0000000040080000..0x0000000040200000 -> 0xffff800040080000..` mapped cleanly; kernel table dump shows the 4K entries
- **Committed in:** `04fcf12` (Task 1 commit)

**3. [Rule 1 - Bug] vmdebug User-table walk faults on virt (user page table does not exist)**
- **Found during:** Task 1 step (7), second boot attempt — clean run through the Kernel recursive-table dump, then `DataAbortSameEl` at FAR `0x0000_ffff_ffff_f000` in `print_table_at_level`
- **Issue:** 03-02 gated `init_user_page_tables()` to `not(virt)` with the user-page-table experiment, but main9's `print_recursive_tables(RootPageTableType::User)` stayed platform-agnostic; on virt it dereferences the user recursive root through TTBR0 (still the early identity map), whose entry[511] is invalid → translation fault
- **Fix:** gate the User walk `#[cfg(not(platform = "virt"))]` with a Phase-4 comment, mirroring the gating of the setup it dumps; the Kernel walk (which proved the DTB/text/data maps in the log) still prints on virt
- **Files modified:** ../r9/aarch64/src/main.rs
- **Verification:** boot reaches "looping now" with zero wedge markers; raspi output unchanged (both walks still print there)
- **Committed in:** `04fcf12` (Task 1 commit)

**4. [Rule 1 - Lint] identity_op clippy warning in my own new virt putc body**
- **Found during:** Task 2 regression sweep (virt clippy) — `PL011_DR = PL011_BASE + 0x00`
- **Issue:** one NEW warning introduced by this plan (all other warnings in touched files are the pre-existing ones 03-02 documented)
- **Fix:** `PL011_DR = PL011_BASE` with the offset-0x00 comment retained; amended into the Task 1 commit (amend happened before any push)
- **Files modified:** ../r9/aarch64/src/pre_mmu/util.rs
- **Verification:** virt clippy green with zero warnings in plan-touched files
- **Committed in:** `04fcf12` (Task 1 commit, amended)

### Process note (not a code deviation)

**5. Boot-harness flake in the plan's verify command — root-caused, no gate substitution**
- **Found during:** Task 1/Task 2 verify runs
- **Issue:** the written `mkdir -p … && qemu … &` binds `&` to the whole list, so `$QP` names a subshell (kill orphans qemu), and with a stale serial log present the first poll (`i=1`, before any sleep) reads pre-boot content — one run failed the DTB-line grep by racing qemu's truncate
- **Handling:** no code change and NO gate substitution — the plan-verbatim command passes deterministically from a fresh log (`rm -f` first), demonstrated; orphan qemu cleaned via the bracket-fixed `pkill -f` idiom from 03-03. Recorded in `.planning/WINDOWS.md` so 03-05's D-11 harness author sees it (the socket-chardev design avoids the pattern)
- **Committed in:** n/a

---

**Total deviations:** 4 auto-fixed (3 Rule 1, 1 Rule 3) + 1 process note
**Impact on plan:** All four code fixes are precisely what the tracer exists to catch — architectural dead-ends found on the boot-proven slice before GIC/timer/harness investment. No scope creep: every fix is inside the plan's file list and boot path; raspi behavior is byte-identical (gated out everywhere); all must_have truths and prohibitions hold.

## Issues Encountered

- **Stale-ELF boot confusion (operator error, caught and corrected):** one boot ran a stale ELF because the three-config build order left the raspi4b binary at the shared target path, and one "green" build actually hid an E0133 compile error (`asm!` outside `unsafe` under `forbid(unsafe_op_in_unsafe_fn)`) behind `tail`-truncated output — caught by disassembling for the missing `msr CPACR_EL1` and re-running with explicit exit-status checks. The incident independently CONFIRMED 03-03's deferred upstream defect: the raspi4b build on the virt machine data-aborts at FAR `0xfe215004` in the raspi early-uart path — the same signature as the pre-existing raspi3b-on-qemu-11.1.2 crash.
- No other issues. Both tasks' written verify gates pass unsubstituted.

## User Setup Required

None - no external service configuration required. (D-10's linux-host verification legs remain queued for end-of-phase UAT from 03-01; this plan adds no new ones.)

## Next Phase Readiness

- **03-05 expands onto a boot-proven slice**: GICv2 + generic timer + UART RX IRQ init land in `platform/virt/` (gic.rs/timer.rs are the planned new files), then call `trap::unmask_irq()` once handlers are armed (the P5 ordering contract from 03-02). The PL011's DTB regblock, the `map_device_register` idiom, and the `arm,cortex-a15-gic` compatible string are all proven live on the real machine.
- **Evidence pattern for 03-05/03-06**: captured-serial gates must start from a fresh log (or use the D-11 socket chardev) — see the WINDOWS.md process note.
- **Deferred-item confirmation**: the raspi-on-qemu-11.1.2 defect (03-03, deferred) reproduced identically from an operator-error boot — signature unchanged (FAR `0xfe215004`).
- R9-01 remains open (shared by plans 03-02…03-06; the shared-ID gate marks it when the last declaring plan completes).

---
*Phase: 03-r9-qemu-virt-platform-bring-up*
*Completed: 2026-11-01*

## Self-Check: PASSED

- FOUND: ../r9/aarch64/lib/config_virt.toml
- FOUND: ../r9/aarch64/lib/virt-gicv2.dts
- FOUND: ../r9/aarch64/lib/virt-gicv2.dtb
- FOUND: ../r9/aarch64/src/platform/virt/mod.rs
- FOUND: ../r9/aarch64/src/platform/virt/pl011.rs
- FOUND: ../r9/aarch64/src/platform/virt/devcons.rs
- FOUND: r9 commit 04fcf12 (`feat(03-04): virt platform tracer — DTB artifacts, config, memory-map fixes, early PL011, banner boots`, branch main)
- FOUND: r9 commit c74fcb1 (`feat(03-04): virt print_machine_info — DTB-derived machine info in the board-info slot (D-03/D-04)`, branch main)
- FOUND: .planning/phases/03-r9-qemu-virt-platform-bring-up/03-04-SUMMARY.md
- kernel.ld diff vs r9 HEAD: empty (UNCHANGED per prohibition)
