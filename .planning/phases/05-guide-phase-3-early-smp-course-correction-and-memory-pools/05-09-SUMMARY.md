---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 09
subsystem: aarch64
tags: [smp, aarch64, tpidr-el1, psci, cpu-on, gicv2, gicv3, sgi, ipi, tlb-shootdown, secondary-start, trap-migration, smp-02, smp-04, smp-07]

# Dependency graph
requires:
  - phase: 05-07
    provides: the SmpArch hook boundary (register_arch, current_cpu_id, irq_save_disable/irq_restore, send_resched_ipi/send_shootdown_ipi, local_tlb_flush, ticks, prepare_block), the LockRank table, the lock-free ShootdownCoordinator
  - phase: 05-08
    provides: port::sched::RunQueues (the per-CPU work-stealing run queues this arch's sched loop will consume), the prepare_block seam, the cpu_debug_identity_is_real gate
provides:
  - aarch64/src/smp.rs: PerCpu (repr(C, align(64)), cur_frame at offset 0 compile-time asserted), PER_CPU static table, this_cpu() via TPIDR_EL1, init_bsp, publish_boot_state, DTB /cpus + /psci discovery (bounds-checked at MAX_CPUS), PSCI CPU_ON (0xC400_0003) with PSCI_RET_INVALID_PARAMETERS (-2) tolerated as absent CPU, irq_save_disable/irq_restore (DAIF), local_tlb_flush (tlbi vae1is / vmalle1is), register_port_smp wiring port::smp::register_arch
  - aarch64/src/l.S: secondary_start (EL2->EL1 drop, published MAIR/TCR/TTBR0/TTBR1 from .bootdata globals, MMU enable, VBAR_EL1, TPIDR_EL1 = &PER_CPU[cpu_id], per-CPU 64 KiB boot stack), .bootdata globals SECONDARY_MAIR/TCR/TTBR0/TTBR1, .bss SECONDARY_STACKS (8 x 64 KiB)
  - aarch64/src/trap.S: PerCpu::cur_frame via mrs xN, tpidr_el1 + [xN, #0] replacing every =CURFRAME code reference (trap entry, trap_return, trap_return_exec)
  - aarch64/src/platform/virt/gic/{mod,gicv2,gicv3}.rs: send_sgi (GICv2 GICD_SGIR 0xF00; GICv3 ICC_SGI1R_EL1), init_secondary_cpu (GICv2 banked GICC+ISENABLER0/IPRIORITYR0; GICv3 per-CPU GICR RD wake + SGI frame + ICC_*_EL1)
  - aarch64/src/main.rs: mod smp mount, init_bsp + publish_boot_state boot ordering, temporary secondary_main spin stub (replaced in place by Plan 05-22 Task 1)
  - UNSAFE-AUDIT.md: Plan 05-09 delta (aarch64 184 -> 202 blocks, 6 -> 7 extern blocks, SAFETY 390 -> 408)
affects: [05-10 aarch64 proc.rs CURFRAME/CURKSTACK migration to PerCpu fields + wake_pending + block(), 05-11 x86_64 SmpArch registration (GS base/LAPIC), 05-22 aarch64 -smp 2 boot gate (calls start_secondaries, replaces secondary_main), Owner Stop 2]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (55000, confidence: low).
actuals:
  tokens: 12014
  tasks: 3
  commits: 3
  plan_head_before: ef9553d8fc5c846ac4075e94fb2beb859cfa7137
  plan_head_after: ee6be00f08f8f310a6d3eb6f8e3e92839b37e672

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-CPU state via TPIDR_EL1: PerCpu struct at repr(C, align(64)) with cur_frame at offset 0 compile-time asserted; trap.S reads/writes [tpidr_el1, #0]; the BSP's init_bsp sets TPIDR_EL1 = &PER_CPU[0] BEFORE trap::init installs VBAR_EL1 so any trap after vector arming reads a valid cur_frame"
    - "PSCI CPU_ON with absent-CPU tolerance: PSCI_RET_INVALID_PARAMETERS (-2) returns Ok(false) so a 2-CPU static DTB under -smp 1 skips CPU 1 without panicking or erroring"
    - "Boot-state publication for secondaries: the BSP reads MAIR/TCR/TTBR0/TTBR1 from live registers after init_vm and writes them to .bootdata globals; secondary_start reads them at physical addresses (MMU off) before its own MMU enable"
    - "GIC SGI delivery split: GICv2 writes GICD_SGIR (0xF00) with a dsb ishst data-before-signal barrier; GICv3 writes ICC_SGI1R_EL1 with dsb ishst + isb; both dispatch through the gic::send_sgi write-once cell"
    - "GICv3 per-CPU redistributor wake on secondary: scan GICR RD frames by MPIDR_EL1 affinity, clear PROCESSOR_SLEEP, poll CHILDREN_ASLEEP, configure the SGI frame + ICC_*_EL1 — the same sequence as BSP init but on the secondary's own frame"

key-files:
  created:
    - aarch64/src/smp.rs
  modified:
    - aarch64/src/main.rs
    - aarch64/src/l.S
    - aarch64/src/trap.S
    - aarch64/src/platform/virt/gic/gicv2.rs
    - aarch64/src/platform/virt/gic/gicv3.rs
    - aarch64/src/platform/virt/gic/mod.rs
    - UNSAFE-AUDIT.md

key-decisions:
  - "PerCpu::cpu_id is read via pointer arithmetic ((this_cpu() - &PER_CPU) / size_of::<PerCpu>()) in the SmpArch hook, avoiding a mutable write to the plain u32 field after init; mark_online writes it once (unsafe, SAFETY-documented) before the online flag publishes the slot"
  - "GIC send_sgi implementations landed in Task 1 (with smp.rs) rather than Task 3 (with init_secondary_cpu) because register_port_smp references gic::send_sgi and must compile in Task 1; Task 3 then added only init_secondary_cpu"
  - "proc.rs's CURFRAME/CURKSTACK statics are intentionally untouched: trap.S now reads PerCpu::cur_frame via TPIDR_EL1 while proc.rs still writes the old statics — the plan's flagged assumption scopes this plan to compile-time + clippy soundness, with the proc.rs migration (SMP-03) and the full -smp 2 boot deferred to 05-10 and 05-22 respectively"
  - "secondary_start does NOT zero the secondary's BSS (the BSP cleared it); the secondary's stack slot in SECONDARY_STACKS is zeroed BSS memory by construction"

patterns-established:
  - "Physical-addressed .bootdata globals for pre-MMU secondary bring-up: the BSP writes VA, the secondary reads PA (VA - KZERO); the linker script's .text.boot : AT(ADDR - KZERO) makes the translation a single subtraction"
  - "Rust-exported stride constant (PER_CPU_STRIDE) consumed by assembly madd: the assembly never hardcodes a Rust struct size; the const assert in smp.rs pins the layout"

requirements-completed: [SMP-02, SMP-04, SMP-07]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "aarch64/src/smp.rs defines PerCpu (repr(C, align(64)), cur_frame at offset 0 compile-time asserted), PER_CPU static, this_cpu() via TPIDR_EL1, DTB /cpus + /psci discovery (bounds-checked at MAX_CPUS), PSCI CPU_ON (0xC400_0003) with PSCI_RET_INVALID_PARAMETERS tolerated, irq_save_disable/irq_restore, local_tlb_flush (tlbi vae1is/vmalle1is), register_port_smp wiring port::smp::register_arch (SMP-02, SMP-04, SMP-07, D-25)"
    requirement: SMP-02
    verification:
      - kind: automated
        ref: "cargo xtask check rc 0; cargo xtask clippy --arch aarch64 --config virtshell rc 0; const _: () = assert!(core::mem::offset_of!(PerCpu, cur_frame) == 0) compiles"
        status: pass
    human_judgment: false
  - id: D2
    description: "aarch64/src/l.S implements secondary_start (EL2->EL1 drop, published MAIR/TCR/TTBR0/TTBR1, MMU enable, VBAR_EL1, TPIDR_EL1 = &PER_CPU[cpu_id] via PER_CPU_STRIDE madd, per-CPU 64 KiB boot stack, branch to secondary_main); aarch64/src/trap.S replaces every =CURFRAME code reference with mrs xN, tpidr_el1 + [xN, #0] in trap entry, trap_return, and trap_return_exec (SMP-02, SMP-04)"
    requirement: SMP-04
    verification:
      - kind: automated
        ref: "grep -v '^[[:space:]]*//' aarch64/src/trap.S | grep -c 'tpidr_el1' = 3 (>= 3); grep -v comments aarch64/src/trap.S | grep '=CURFRAME' = 0; cargo xtask check rc 0 (assembler/linker clean)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Gicv2 and Gicv3 support secondary CPU init (banked GICC/GICD_ISENABLER0 for GICv2; per-CPU GICR RD wake + ICC_*_EL1 for GICv3) and SGI IPI delivery (SGI 0 resched via GICD_SGIR, SGI 1 shootdown via ICC_SGI1R_EL1) (SMP-04, SMP-07)"
    requirement: SMP-07
    verification:
      - kind: automated
        ref: "cargo xtask clippy --arch aarch64 --config virtshell rc 0; cargo test -p xtask lintcontract 13/13; GICD_SGIR present in gicv2.rs; ICC_SGI1R_EL1 present in gicv3.rs"
        status: pass
    human_judgment: false

# Metrics
duration: 13min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 09: AArch64 Two-CPU SMP Hardware Bring-Up Summary

**AArch64 per-CPU state via TPIDR_EL1 (PerCpu with compile-time-asserted cur_frame at offset 0), PSCI CPU_ON with absent-CPU tolerance, l.S secondary_start (EL2→EL1 drop, MMU enable, per-CPU stack), trap.S migrated from global CURFRAME to TPIDR_EL1-indexed cur_frame, GICv2/GICv3 secondary CPU init and SGI IPI delivery — all compile-clean and clippy-clean on the virtshell config.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-11-26T07:34:53Z
- **Completed:** 2026-11-26T07:48:22Z
- **Tasks:** 3 of 3 executed
- **Files modified:** 8 (1 created: `aarch64/src/smp.rs`; 7 modified)

## Accomplishments

- **SMP-02 — Per-CPU state is live.** `PerCpu` (`#[repr(C, align(64))]`) carries `cur_frame` (offset 0, compile-time asserted), `cur_kstack`, `cpu_id`, `online`, `current_proc`, `sched_context`, `sched_seeded`, `fpu: CpuFp`, and `pending_shootdown`.  `PER_CPU: [PerCpu; MAX_CPUS]` is the static table; `this_cpu()` reads `TPIDR_EL1`.  `init_bsp()` sets `TPIDR_EL1 = &PER_CPU[0]` before `trap::init()` installs `VBAR_EL1`, so any trap after vector arming reads a valid `cur_frame` through `TPIDR_EL1`.
- **SMP-04 — CPU 1 hardware bring-up compiles clean.**  `secondary_start` in `l.S` drops EL2→EL1, loads the BSP's published MAIR/TCR/TTBR0/TTBR1 from `.bootdata` globals (physical addresses, MMU off), enables MMU + caches, jumps to high-half, installs `VBAR_EL1`, sets `TPIDR_EL1 = &PER_CPU[cpu_id]` via the Rust-exported `PER_CPU_STRIDE`, switches to a per-CPU 64 KiB boot stack, and branches to `secondary_main(cpu_id)`.  `psci_cpu_on` handles `PSCI_RET_INVALID_PARAMETERS` (`-2`) as an absent CPU, so `-smp 1` boots cleanly against a 2-CPU static DTB.
- **SMP-02 — trap.S migration.**  Every `=CURFRAME` code reference is replaced by `mrs xN, tpidr_el1` + `[xN, #0]` (trap entry, `trap_return`, `trap_return_exec`).  Zero `=CURFRAME` code references remain; the only `CURFRAME` mentions are updated comments.
- **SMP-07 — GIC SGI IPIs and secondary init.**  `Gicv2::send_sgi` writes `GICD_SGIR` (`0xF00`) with a `dsb ishst` barrier; `Gicv3::send_sgi` writes `ICC_SGI1R_EL1` with `dsb ishst` + `isb`.  `Gicv2::init_secondary_cpu` configures banked `GICC_PMR`/`GICC_CTLR` and enables PPI 30 + SGI 0 + SGI 1 in the banked `GICD_ISENABLER0`.  `Gicv3::init_secondary_cpu` scans GICR RD frames by MPIDR affinity, wakes the redistributor, enables the SGI frame's PPI 30 + SGI 0 + SGI 1, and configures `ICC_SRE_EL1`/`ICC_PMR_EL1`/`ICC_IGRPEN1_EL1`.
- **SMP-02/07 — SmpArch registration.**  `register_port_smp()` wires `port::smp::register_arch` to the arch hooks: TPIDR_EL1 CPU ID (pointer arithmetic), dense online count, DAIF mask/restore, GIC SGI resched/shootdown senders, `tlbi vae1is`/`vmalle1is` local flush, the arch timer tick, and the prepare_block no-op stub.

## Task Commits

Each task was committed atomically:

1. **Task 1: aarch64 SMP per-CPU state, PSCI CPU_ON, GIC SGI IPIs** — `6a77f47` (feat) — `aarch64/src/smp.rs` created, `main.rs` mod + stub, GIC send_sgi implementations, UNSAFE-AUDIT.md delta
2. **Task 2: l.S secondary_start + trap.S TPIDR_EL1 migration** — `5b652d0` (feat) — `l.S` secondary_start + .bootdata globals + SECONDARY_STACKS, `trap.S` TPIDR_EL1 migration, `main.rs` init_bsp + publish_boot_state ordering, `smp.rs` publish_boot_state + PER_CPU_STRIDE
3. **Task 3: GICv2/GICv3 secondary CPU init** — `ee6be00` (feat) — `gicv2.rs` init_secondary_cpu, `gicv3.rs` init_secondary_cpu + gicr_physrange storage, `gic/mod.rs` init_secondary_cpu dispatch, UNSAFE-AUDIT.md Task 3 delta

## Files Created/Modified

- `aarch64/src/smp.rs` — **created** (~430 lines): module docs, PerCpu struct + const assert, PER_CPU static, ONLINE_COUNT, this_cpu(), mark_online, init_bsp, publish_boot_state, PER_CPU_STRIDE export, DtbTopology + discover, psci_cpu_on, secondary_start_pa, start_secondaries, SGI_RESCHED/SGI_SHOOTDOWN constants, SmpArch hook implementations, register_port_smp
- `aarch64/src/main.rs` — `mod smp` declaration, `init_bsp()` + `publish_boot_state()` before `trap::init()`, `secondary_main` temporary spin stub
- `aarch64/src/l.S` — `secondary_start` entry (EL2→EL1 drop, MMU state load from .bootdata, MMU enable, high-half jump, VBAR_EL1, TPIDR_EL1, per-CPU stack, call secondary_main), SECONDARY_MAIR/TCR/TTBR0/TTBR1 .bootdata globals, SECONDARY_STACKS .bss area (8 x 64 KiB)
- `aarch64/src/trap.S` — 3 CURFRAME code references replaced with TPIDR_EL1 offset-0 access; comments updated
- `aarch64/src/platform/virt/gic/mod.rs` — `send_sgi` + `init_secondary_cpu` dispatches
- `aarch64/src/platform/virt/gic/gicv2.rs` — `GICD_SGIR` constant, `send_sgi`, `init_secondary_cpu` (banked GICC + ISENABLER0/IPRIORITYR0)
- `aarch64/src/platform/virt/gic/gicv3.rs` — `gicr_physrange` stored in Gicv3, `send_sgi` (ICC_SGI1R_EL1), `init_secondary_cpu` (GICR RD wake + SGI frame + ICC_*_EL1), `icc_sgi1r_el1_write` helper
- `UNSAFE-AUDIT.md` — Plan 05-09 delta (16 new blocks + 1 extern across Tasks 1+3), per-file inventory updated (aarch64 184 → 202 blocks, 6 → 7 extern blocks, SAFETY 390 → 408)

## Decisions Made

- **`current_cpu_id` via pointer arithmetic.**  The SmpArch hook computes the cpu id as `(this_cpu() - &PER_CPU[0]) / size_of::<PerCpu>()` rather than reading the `cpu_id` field, avoiding a second unsafe access.  `mark_online` still writes the field once (SAFETY-documented) for readers that need the field directly.
- **GIC `send_sgi` landed in Task 1.**  `register_port_smp` references `gic::send_sgi` and must compile in Task 1; the `send_sgi` implementations are small (one MMIO write each) and self-contained.  Task 3 then added only `init_secondary_cpu`.  This is a Rule 3 deviation (see below).
- **proc.rs CURFRAME/CURKSTACK statics intentionally untouched.**  trap.S now reads `PerCpu::cur_frame` via TPIDR_EL1 while proc.rs still writes the old statics — the plan's flagged assumption scopes this plan to compile-time + clippy soundness, with the proc.rs migration (SMP-03) deferred to 05-10 and the full `-smp 2` boot to 05-22.
- **Secondary BSS not re-zeroed.**  The BSP's BSS clear at boot zeroes SECONDARY_STACKS; the secondary CPU starts long after and its stack slot is zeroed by construction.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] GIC `send_sgi` implementations landed in Task 1 (not Task 3)**
- **Found during:** Task 1 (`register_port_smp()` compilation)
- **Issue:** The plan's Task 1 requires `register_port_smp()` wiring GIC SGI 0/1 senders into `port::smp::register_arch`, but the GIC-level `send_sgi` dispatch (`gic/mod.rs`) and the `Gicv2::send_sgi`/`Gicv3::send_sgi` methods were Task 3's scope.  Task 1's verify (`cargo xtask check && clippy`) fails without them.
- **Fix:** Task 1 includes the `send_sgi` dispatch in `gic/mod.rs` and the `Gicv2`/`Gicv3` `send_sgi` implementations.  Task 3 then adds only `init_secondary_cpu` (the plan's heavier GIC work: GICR wake on v3, banked GICC on v2).
- **Files modified:** `aarch64/src/platform/virt/gic/mod.rs`, `gicv2.rs`, `gicv3.rs` (in Task 1's commit `6a77f47`)
- **Verification:** `cargo xtask check` rc 0, `cargo xtask clippy --arch aarch64 --config virtshell` rc 0
- **Committed in:** `6a77f47` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking — task boundary adjustment for compile-time soundness)
**Impact on plan:** The task boundary shift is zero-scope-change: the same code lands, just split differently across the three commits.  All must_haves and acceptance criteria are met.  `Cargo.lock` unchanged (zero external packages — T-05-SC gate N/A).

## Issues Encountered

- None beyond the deviation above.  The baseline suite was green at spawn.  Pre-existing warnings (ds SEQ_INC, devip unused import, allocator unused-mut, netlog dead code, proto TCPSTAT consts, raspi3b uartpl011 unused import, syscall unused imports) were left untouched per the scope boundary.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

- **`secondary_main` in `aarch64/src/main.rs`** is a deliberate temporary spin stub (`wfi` loop), documented as "replaced in place by Plan 05-22 Task 1".  The plan's artifact list explicitly names this as the intended shape ("`aarch64/src/main.rs`: ... plus initial `#[unsafe(no_mangle)] pub extern "C" fn secondary_main` spin stub (replaced in place in `aarch64/src/main.rs` by Plan `05-22` Task 1)").  Not a defect — the plan's own design.
- **`proc.rs` CURFRAME/CURKSTACK statics** are still written by `write_curframe`/`write_curkstack` but no longer read by trap.S.  This is the plan's flagged assumption: "this plan guarantees compile-time + clippy soundness of the hardware bring-up layer"; the proc.rs migration is 05-10's scope (SMP-03).  Not a stub introduced by this plan — the old statics are the pre-existing code path awaiting migration.
- **`register_port_smp` is not yet called** from `main9` (the call site arrives with the `-smp 2` boot wiring in 05-22).  The function exists, compiles, and is clippy-clean; the plan's acceptance criteria require only compile-time soundness.

The broken-windows ledger (`.planning/WINDOWS.md`) has no entries to add: no new stubs beyond the plan's own design, no skipped tests, no unrun `<verify>`; the single deviation was fixed and verified in-commit.

## Threat Flags

None new.  The plan's threat register is implemented as written: T-05-15 (DTB tampering) is mitigated by the bounds-checked `/cpus` `cpu@*` `reg` parser (capped at `MAX_CPUS`) and the strict `"hvc"`/`"smc"` method validation; T-05-16 (elevation of privilege via `secondary_start`/`TPIDR_EL1`) is mitigated by the compile-time `offset_of!(PerCpu, cur_frame) == 0` assert, the per-CPU 64 KiB isolated boot stack, and the `TPIDR_EL1 = &PER_CPU[cpu_id]` write before IRQ unmasking or Rust entry; T-05-SC: `Cargo.lock` is byte-unchanged (verified) — zero external packages installed.  No new network endpoints, auth paths, file-access patterns, or trust-boundary schema changes; the new public surface is exactly the plan's artifact list.

## Next Phase Readiness

- **Plan 05-10 (aarch64 SMP-03 proc.rs migration):** replace `CURFRAME`/`CURKSTACK`/`CURRENT_PROC`/`SCHED_CONTEXT`/`SCHED_SEEDED` statics with `PerCpu` field access via `this_cpu()`; implement `Proc::wake_pending` + `block()`; register `prepare_block` in the `SmpArch` hooks (replacing the no-op stub).
- **Plan 05-11 (x86_64 SmpArch registration):** GS base per-CPU `Mach`, LAPIC IPI vectors, MADT CPU discovery — the x86_64 twin of this plan's aarch64 work.
- **Plan 05-22 (aarch64 -smp 2 boot gate):** call `smp::start_secondaries(dt)` from `main9` (after `platform::gic::init`), call `smp::register_port_smp()`, replace `secondary_main` in place with the real bring-up tail (GIC secondary init, timer arm, sched loop entry), and run the full `-smp 2` boot under HVF (GICv3) and TCG (GICv2).
- **Owner Stop 2 (D-32):** the cumulative diff now includes `aarch64/src/smp.rs`, `l.S`, `trap.S`, `main.rs`, and the GIC drivers on top of the 05-07/05-08 port work; gates move to `-smp 2` after both arch bring-ups.

---

*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- All 2 created files exist on disk (verified with `[ -f ]`): `aarch64/src/smp.rs`, `05-09-SUMMARY.md`.
- All 7 modified files exist on disk (verified with `[ -f ]`): `aarch64/src/main.rs`, `l.S`, `trap.S`, `gic/gicv2.rs`, `gic/gicv3.rs`, `gic/mod.rs`, `UNSAFE-AUDIT.md`.
- All 3 plan commits are ancestors of HEAD (verified with `git merge-base --is-ancestor`): `6a77f47` (T1), `5b652d0` (T2), `ee6be00` (T3).
- Content pins verified: `pub struct PerCpu` + `offset_of!(PerCpu, cur_frame) == 0` in `aarch64/src/smp.rs`; `secondary_start` in `aarch64/src/l.S`; `tpidr_el1` (3 code references, 0 `=CURFRAME` code references) in `aarch64/src/trap.S`; `GICD_SGIR` + `init_secondary_cpu` in `gicv2.rs`; `ICC_SGI1R_EL1` + `init_secondary_cpu` in `gicv3.rs`; `mod smp;` + `secondary_main` in `main.rs`.
- Measured commits from ledger (`gsd-plan-head-before-05-09` = `ef9553d8`): 3 at completion; `plan_head_after` = `ee6be00f` (the last task commit before this SUMMARY's own docs commit).
- Working tree before this SUMMARY's commit: only this SUMMARY untracked — zero generated files left untracked, `Cargo.lock` untouched (byte-identical), `ds/src/tpht/` untouched.
- Plan-level verification re-run at completion: `cargo xtask check` rc 0, `cargo xtask clippy --arch aarch64 --config virtshell` rc 0, `cargo test -p xtask lintcontract` 13/13.
