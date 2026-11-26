---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 11
subsystem: x86_64-smp
tags: [smp, x86_64, q35, acpi, madt, lapic, x2apic, icr, init-sipi-sipi, ap-trampoline, mach, gs-base, intrspecial, tlb-shootdown, ipi, smp-01, smp-02, smp-07, tdd]

# Dependency graph
requires:
  - phase: 05-07
    provides: the SmpArch hook boundary (register_arch, current_cpu_id, irq_save_disable/irq_restore, send_resched_ipi/send_shootdown_ipi, local_tlb_flush, ticks, prepare_block), the LockRank table, the lock-free ShootdownCoordinator (service_shootdown_ipi)
  - phase: 05-22
    provides: the proven aarch64 two-CPU reference (per-CPU state via TPIDR_EL1, SGI IPI dispatch, boot shootdown self-check shape) this plan re-keys onto x86_64 (D-25)
provides:
  - "x86_64/src/platform/q35/acpi.rs: MADT type-0 (LAPIC) + type-9 (x2APIC) enabled/online-capable CPU enumeration into Madt.cpus (MAX_CPUS cap, dropped_cpus counted), bounds-checked, host-pinned"
  - "x86_64/src/platform/q35/lapic.rs: dual xAPIC (ICR_HI/ICR_LO + delivery-status poll) and x2APIC (mfence;lfence before wrmsr(0x830), SDM 10.12.9, T-05-19) send_init/send_sipi/send_ipi, current_apic_id, init_ap (AP LAPIC enable + ESR clear + timer armed from the BSP's published calibration)"
  - "x86_64/src/ap_trampoline.S: position-independent 16->32->64-bit SIPI entry for PA 0x8000 with the 0x8F00 mailbox (pml4_pa, stack top, Mach VA, cpu id, ap_main VA); 230 bytes, byte-verified"
  - "x86_64/src/smp.rs: install_trampoline, start_aps (INIT->10ms->SIPI(0x08)->200us->retry SIPI->online poll, delays from the BSP's TSC calibration), the AP_MACH_VAS registry, pushfq/cli irq-save + test/sti restore, invlpg/CR3 flush, register_port_smp wiring %gs:0 cpu id + LAPIC 0xF0/0xF1 IPIs"
  - "x86_64/src/dat.rs: Mach.me:u64 pinned at offset 0, per-CPU fields (cpu_id, apic_id, online, cur_frame, current_proc, sched_context, sched_seeded, pending_shootdown), mach_ptr()/this_mach() via %gs:0, init_ap_identity"
  - "x86_64/src/trap.rs: canonical intrspecial bit-63 sign test (testq %rax,%rax; js) replacing the hardcoded KMACH compare (T-05-20), and dispatch of LAPIC vectors 0xF0 (shootdown) and 0xF1 (resched)"
affects: [05-12 (x86_64 SMP-03 migration + ap_main replacement + start_aps wiring), 05-14 (Owner Stop 2 boot verification under CI TCG and owner KVM), SMP-08 stress gate]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate (55000, confidence: low).
actuals:
  tokens: 19614
  tasks: 3
  commits: 4
  plan_head_before: d399118444a6dacd040fa13000a62420a6132764
  plan_head_after: 70179f25f9d2f563992a02ee4f85e87b57fdc592

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "MADT CPU enumeration discipline: include iff (flags & 1) != 0 || (flags & 2) != 0 (enabled or online-capable); overflow clamps at MAX_CPUS with dropped_cpus counted loudly, never a silent truncation and never a boot failure on big firmware tables (T-05-21)"
    - "x2APIC ICR write ordering: mfence; lfence before wrmsr(0x830) in ONE asm block with no nomem claim (the compiler cannot hoist a store past it), per Intel SDM Vol 3A §10.12.9 (T-05-19)"
    - "SIPI trampoline contract: position-independent .rodata bytes copied to PA 0x8000, all internal references 0x8000-relative (never the link address), mailbox at 0x8F00 written volatile per start, Release fence before INIT; the AP's CR3 is the BSP's bootpml4 (identity first 4 GiB keeps the trampoline executing at PA==VA)"
    - "Per-CPU Mach identity: me:u64 at offset 0 (compile-time pinned), IA32_GS_BASE = &mach per CPU, mach_ptr() = mov %gs:0 — and intrspecial's swapgs decision is the bit-63 sign test, valid for EVERY kernel Mach VA (the hardcoded-KMACH compare could never recognize an AP)"
    - "Dead-strip reality: the trampoline + start_aps chain is GC'd from the linked kernel until a live caller anchors it (verified: black_box(start_aps) pulls in trampoline + ap_main + strings); 05-12's call sites are the anchor"

key-files:
  created:
    - x86_64/src/ap_trampoline.S
    - x86_64/src/smp.rs
    - .planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-11-RED-task1.json
  modified:
    - x86_64/src/platform/q35/acpi.rs
    - x86_64/src/platform/q35/lapic.rs
    - x86_64/src/platform/q35/timer.rs
    - x86_64/src/dat.rs
    - x86_64/src/trap.rs
    - x86_64/src/main.rs
    - UNSAFE-AUDIT.md

key-decisions:
  - "The dat.rs Mach work (me at offset 0, the per-CPU fields, mach_ptr/this_mach) landed in Task 2's commit, not Task 3's: start_aps writes those very fields and the SmpArch hook reads %gs:0 cpu_id, so the plan's file split could not compile as ordered (Rule 3 seam fold; both tasks' acceptance criteria verified on the final tree)"
  - "The AP's Mach is a zeroed 64 KiB-aligned heap block (the arena is .bss in the shared kernel half — mapped under every CR3, no PML4 cloning, the RFMEM hazard avoided); its boot stack is its own KStack (the CPU-0 l.S shape re-keyed)"
  - "The trampoline's CR4 sets PAE|PGE|OSFXSR|OSXMMEXCPT — BSP parity (l.S arms the same set minus FSGSBASE); an AP must not join the kernel with a weaker CR4 than CPU 0 (T-05-23's identically-to-BSP discipline)"
  - "init_ap never re-calibrates: it arms the AP's LAPIC timer from the BSP's published TimerState (timer.rs gained a state() getter + Copy for it — Rule 2 seam; the plan's text mandates reusing the calibration)"

patterns-established:
  - "BSP/AP platform parity: AP bring-up mirrors the BSP's CR4/EFER/LAPIC mode exactly (CPUID-gated x2APIC switch on both), never a reduced secondary profile"
  - "Boot-delay arithmetic from the published timer calibration (TICK_MS = 10 ms, so tsc_delta IS 10 ms of TSC ticks; 200 us = tsc_delta/50) — no second calibration, no compiled-in frequencies"

requirements-completed: [SMP-01, SMP-02, SMP-07]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "acpi.rs records enabled/online-capable MADT type-0 (LAPIC) and type-9 (x2APIC) CPUs with exact APIC IDs, bounds-checked entries (elen >= 8 / elen >= 16), and a counted MAX_CPUS cap (SMP-01, T-05-21)"
    requirement: SMP-01
    verification:
      - kind: unit
        ref: "x86_64/src/platform/q35/acpi.rs tests: madt_records_enabled_lapic_cpus_and_skips_disabled, madt_records_online_capable_cpus, madt_x2apic_entries_extract_32bit_ids_and_reject_malformed, madt_cpu_overflow_is_capped_and_counted — compile-proven on this host by error-diffing cargo check --tests (zero diagnostics reference the new code); assertion-level execution is scoped by the plan to x86_64 hosts (cargo xtask test's x86_64 leg runs on CI x86_64 runners; see Environment notes)"
        status: unknown
      - kind: automated
        ref: "nix develop -c cargo xtask check rc 0; cargo xtask clippy --arch x86-64 --config q35cpu rc 0; cargo xtask test rc 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "lapic.rs delivers xAPIC (ICR_HI/ICR_LO + delivery-status poll) and x2APIC (mfence;lfence + wrmsr(0x830)) send_init/send_sipi/send_ipi, current_apic_id (EXTD-gated MSR 0x802 vs MMIO ID>>24), and init_ap (AP enable + ESR clear + timer from BSP calibration) (SMP-01, SMP-07, T-05-19)"
    requirement: SMP-07
    verification:
      - kind: unit
        ref: "x86_64/src/platform/q35/lapic.rs test icr_x2apic_value_packs_destination_in_upper_half (same host constraint as D1)"
        status: unknown
      - kind: automated
        ref: "q35cpu clippy rc 0; full kernel build (x86_64-unknown-none-elf) rc 0; icr packing pinned by the host test on CI"
        status: pass
    human_judgment: false
  - id: D3
    description: "ap_trampoline.S assembles cleanly via global_asm! at 230 bytes (< 0xF00, never overlapping the 0x8F00 mailbox) and smp.rs implements install_trampoline + the INIT->10ms->SIPI->200us->retry->online-poll start_aps handshake plus register_port_smp (SMP-01, SMP-02)"
    requirement: SMP-01
    verification:
      - kind: automated
        ref: "full build rc 0; object carries ap_trampoline_start/end at 0x0..0xe6; byte-level disassembly review of the emitted encodings (lgdtl/far jumps/CR3/EFER/CR0/mailbox reads all hand-verified); dead-strip-until-called mechanism probed and confirmed (black_box(start_aps) anchors trampoline+ap_main into the linked kernel)"
        status: pass
    human_judgment: false
  - id: D4
    description: "trap.rs intrspecial tests IA32_GS_BASE bit 63 (testq %rax,%rax; js) instead of hardcoded KMACH, and dispatch_irq arms LAPIC 0xF0 (port::smp::service_shootdown_ipi) and 0xF1 (proc::preempt) with the shared EOI tail (SMP-02, SMP-07, T-05-20)"
    requirement: SMP-02
    verification:
      - kind: automated
        ref: "q35cpu clippy rc 0; the sign-test sequence verified in the LINKED kernel (0f 32; 48 c1 e2 20; 48 09 d0; 48 85 c0; pops; js +3 skipping the 3-byte swapgs); single-CPU behavior equivalence: KMACH has bit 63=1 and user GS base is always 0 on r9"
        status: pass
    human_judgment: false
  - id: D5
    description: "Full x86_64 -smp 2 boot proof (CPU 1 online + rio green) — scheduled at Plan 05-14 / Owner Stop 2 on CI TCG and the owner's Linux KVM host per the plan's flagged assumption (D-32, D-33)"
    requirement: SMP-01
    verification: []
    human_judgment: true
    rationale: "x86_64 boots cannot run on the aarch64 macOS dev host (grub-mkrescue and the authdomain generator are Linux-only in this toolchain — pre-existing environment constraint, per D-33); this plan's own flagged assumption defers the live -smp 2 boot verification to Plan 05-14"

# Metrics
duration: 59min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 11: x86_64 Two-CPU SMP Hardware Bring-Up Summary

**x86_64 q35 gains ACPI MADT CPU enumeration (type-0 LAPIC + type-9 x2APIC with a counted MAX_CPUS cap), dual xAPIC/x2APIC ICR IPI delivery (with the SDM §10.12.9 mfence;lfence ordering), a position-independent 16→32→64-bit SIPI trampoline at PA 0x8000 with an 0x8F00 mailbox, per-CPU `Mach` state via `%gs:0` with `me` pinned at offset 0, the canonical `intrspecial` bit-63 `swapgs` test, and LAPIC IPI vectors 0xF0/0xF1 dispatching to the port shootdown coordinator and the scheduler — all compiling clean and passing `q35cpu` clippy, ready for 05-12's runtime wiring (SMP-01, SMP-02, SMP-07, D-25).**

## Performance

- **Duration:** 59 min
- **Started:** 2026-11-26T15:57:33Z
- **Completed:** 2026-11-26
- **Tasks:** 3 of 3 executed
- **Files modified:** 10 (2 created: `x86_64/src/ap_trampoline.S`, `x86_64/src/smp.rs`; 7 modified; 1 evidence JSON)

## Accomplishments

- **SMP-01 — MADT CPU discovery is real and hostile-table-safe.**  `Madt` now carries `cpus`/`ncpus`/`dropped_cpus`: type-0 LAPIC entries (`elen >= 8`) contribute their 8-bit APIC ID and type-9 x2APIC entries (`elen >= 16`) their full 32-bit ID when enabled (bit 0) or online-capable (bit 1); disabled entries are skipped, malformed/truncated entries are the file's loud `BadEntry` (never a panic, never a partial `Madt`), and an overflow clamps at `MAX_CPUS` with the drop counted for the caller to print.  Four host pins cover the enabled+disabled filter, online-capable inclusion, 32-bit ID extraction, malformed rejection, and the cap.
- **SMP-01/07 — dual ICR IPI delivery.**  `lapic::send_init`/`send_sipi`/`send_ipi` ride one `send_icr`: xAPIC writes ICR_HI then ICR_LO and polls delivery-status bit 12 (bounded, loud on a wedge); x2APIC executes `mfence; lfence` then `wrmsr(0x830, (apic_id << 32) | delivery)` in one compiler-barriered asm block per Intel SDM Vol 3A §10.12.9 (T-05-19).  `current_apic_id()` is hardware-derived (EXTD-gated MSR 0x802 vs MMIO ID >> 24 — correct on an AP pre-init and the BSP post-init), and `init_ap()` enables an AP's LAPIC in the platform's mode, clears ESR, and arms its LAPIC timer from the BSP's published calibration (never a re-calibration).
- **SMP-01/02 — the SIPI trampoline and the per-CPU Mach.**  `ap_trampoline.S` (230 bytes, assembled into `.rodata`) runs 16→32→64 entirely from its copy at PA 0x8000: embedded GDT, BSP-parity CR4/EFER, CR3 from the mailbox's `pml4_pa`, then stack + args + the jump to `ap_main` from the 0x8F00 mailbox — every encoding hand-verified against the emitted object.  `smp.rs`'s `start_aps` allocates each AP a zeroed, 64 KiB-aligned `Mach` on the heap (the shared kernel half — distinct VA per CPU, zero PML4 cloning), publishes the mailbox, and runs the SDM §8.4.4 INIT→10 ms→SIPI→200 µs→retry→online-poll handshake with delays derived from the BSP's TSC calibration.  `Mach` now carries `me: u64` at offset 0 (compile-time pinned) plus the per-CPU `cpu_id`/`apic_id`/`online`/`cur_frame`/`current_proc`/`sched_context`/`sched_seeded`/`pending_shootdown` fields, and `dat::mach_ptr()`/`this_mach()` read `%gs:0`.
- **SMP-02/07 — the canonical trap entry and IPI dispatch.**  `intrspecial` replaces the hardcoded `movabsq $KMACH; cmpq; je` with the kernel-half sign test (`testq %rax, %rax; js`) — verified byte-exact in the linked kernel (`js +3` skips the 3-byte `swapgs`) — so every kernel Mach VA is recognized on every CPU while every user GS base (always 0 on r9: closed syscall list, no FSGSBASE) swaps (T-05-20).  `dispatch_irq` arms `0xF0` → `port::smp::service_shootdown_ipi()` and `0xF1` → `proc::preempt()`, retired by the shared LAPIC EOI tail; the IDT already gated all 256 vectors.
- **UNSAFE-AUDIT.md moved in lockstep per commit (D-03):** x86_64 156 → 178 blocks, extern 2 → 3, SAFETY 402 → 424, with per-file rows updated and one stale-baseline correction recorded.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: failing pins for MADT CPU enumeration + x2APIC ICR packing** — `5aab161` (test)
2. **Task 1 GREEN: MADT CPU enumeration + dual xAPIC/x2APIC ICR IPIs** — `d3a0b6c` (feat)
3. **Task 2: SIPI trampoline, per-CPU Mach via %gs:0, and smp.rs start_aps** — `b2ab7bb` (feat)
4. **Task 3: canonical intrspecial GS sign test + IPI vectors 0xF0/0xF1** — `70179f2` (feat)

**Plan metadata:** this SUMMARY's own commit (docs) follows.

## Files Created/Modified

- `x86_64/src/ap_trampoline.S` — the 16→32→64-bit position-independent SIPI entry (copied to PA 0x8000), embedded GDT, 0x8F00 mailbox contract; 230 bytes
- `x86_64/src/smp.rs` — trampoline install, `start_aps` INIT-SIPI-SIPI handshake, `AP_MACH_VAS` registry, IRQ save/restore, `local_tlb_flush` (`invlpg`/CR3 reload), `register_port_smp`, vectors `0xF0`/`0xF1`
- `x86_64/src/platform/q35/acpi.rs` — `MadtCpu`, `Madt::cpus`/`ncpus`/`dropped_cpus`, type-0/type-9 recording arms, 4 host pins
- `x86_64/src/platform/q35/lapic.rs` — ICR constants, `icr_x2apic_value`, `send_icr` (dual-mode), `send_init`/`send_sipi`/`send_ipi`, `current_apic_id`, `init_ap`, the ICR packing pin
- `x86_64/src/platform/q35/timer.rs` — `TimerState: Copy`, `state()` getter, `pub(crate) MSR_IA32_TSC_DEADLINE` (the AP arming seam)
- `x86_64/src/dat.rs` — `Mach` layout: `me: u64` at offset 0 (pinned), per-CPU SMP fields, `mach_ptr()`/`this_mach()`, `init_ap_identity`; `FromZeros` marker dropped (see Deviations 4)
- `x86_64/src/trap.rs` — `intrspecial` canonical sign test, IPI dispatch arms, doc-comment rewrite of the GS-base rationale
- `x86_64/src/main.rs` — `mod smp;` mount + the temporary `ap_main` stub (05-12 replaces it in place)
- `UNSAFE-AUDIT.md` — the three per-task 05-11 deltas + per-file inventory rows
- `.planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-11-RED-task1.json` — the RED evidence record (see Environment Constraints)

## Decisions Made

- **The Mach seam folded forward into Task 2.**  The plan lists the `dat.rs` work under Task 3, but Task 2's `start_aps` writes `mach.me`/`cpu_id`/`apic_id` and its SmpArch hook reads the `%gs:0` cpu id — the split could not compile as ordered.  Rule 3 fold: Task 2's commit carries the `dat.rs` surface; Task 3's commit carries `trap.rs` + the audit.  Both tasks' acceptance criteria were verified on the final tree.
- **Heap-allocated AP Mach, its own KStack as the boot stack.**  The arena is a `.bss` static in the shared kernel half, so the block is mapped under every CR3 with no PML4 cloning (the RFMEM hazard the research names); the AP's boot stack is its Mach's own 64 KiB KStack — the exact shape CPU 0 uses from l.S.
- **BSP-parity CR4 on the AP.**  The plan's trampoline text names `CR4.PAE`; the implementation also sets PGE|OSFXSR|OSXMMEXCPT, mirroring l.S's BSP CR4 (minus FSGSBASE, as on the BSP) — an AP must never run with a weaker CR4 than the kernel it joins (T-05-23's identical-to-BSP discipline; `fxsave` on the AP would otherwise fault).
- **MADT overflow clamps and counts rather than erroring.**  "Capped at MAX_CPUS" reads as clamp + `dropped_cpus` (never silent, T-05-21); a hard `TooManyEntries` would fail the whole boot on a >64-CPU firmware table.
- **Timer seam:** an AP arms from `timer::state()` verbatim (TscDeadline: same delta via MSR 0x6E0; Periodic: same DCR divide + ICR count) — no second PIT/TSC measurement anywhere in the bring-up.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] dat.rs's Task-3 surface landed in Task 2's commit**
- **Found during:** Task 2 (smp.rs implementation)
- **Issue:** The plan's task split puts `Mach.me`-at-offset-0, the per-CPU fields, and `mach_ptr()` in Task 3 (`dat.rs`), but Task 2's specified `start_aps` writes `mach.me`/`mach.cpu_id`/`mach.apic_id`/`mach.online` and its `register_port_smp` wires the `%gs:0` CPU-ID hook — Task 2 could not compile without Task 3's dat.rs surface.
- **Fix:** Folded the dat.rs work into Task 2's commit (`b2ab7bb`); Task 3's commit (`70179f2`) carries trap.rs + the audit.  Task 3's dat.rs acceptance criterion (`offset_of!(Mach, me) == 0` + `%gs:0` `this_mach()`) is verified on the final tree.
- **Files modified:** `x86_64/src/dat.rs`, `x86_64/src/smp.rs`
- **Verification:** `cargo xtask check` rc 0 at every commit; acceptance greps pass on the final tree
- **Committed in:** `b2ab7bb`

**2. [Rule 2 - Missing Critical] timer.rs needed a published-state seam for init_ap**
- **Found during:** Task 1 (lapic.rs `init_ap`)
- **Issue:** The plan mandates arming the AP's LAPIC timer "from BSP's calibrated initial count without re-running PIT/TSC calibration", but `TimerState` had no accessor (the cell is private to timer.rs) and timer.rs is outside Task 1's `<files>`.
- **Fix:** Added `#[derive(Clone, Copy)]` on `TimerState`, a `pub(crate) fn state()` getter (publish-once read discipline), and `pub(crate)` on `MSR_IA32_TSC_DEADLINE` — the minimal seam, documented in the audit delta.
- **Files modified:** `x86_64/src/platform/q35/timer.rs`
- **Verification:** q35cpu clippy rc 0; the getter carries the publish-once SAFETY contract
- **Committed in:** `d3a0b6c`

**3. [Rule 1 - Bug] The trampoline's compile-time size assert could not assemble**
- **Found during:** Task 2 (first full build of ap_trampoline.S)
- **Issue:** The plan's "Assert at compile/runtime that `ap_trampoline_end - ap_trampoline_start < 0xF00`" was written as an `.if` over the same-section symbol difference; LLVM MC evaluates `.if` at parse time, before section layout assigns offsets, so the difference is never an absolute expression there (`expected absolute expression` at assembly).
- **Fix:** Dropped the `.if` (a comment records why); the runtime half — `install_trampoline`'s `assert!(len < 0xF00)` on the copied span — carries the guarantee, which the plan's "compile/runtime" phrasing permits.  The measured span is 230 bytes.
- **Files modified:** `x86_64/src/ap_trampoline.S`, `x86_64/src/smp.rs`
- **Verification:** full `cargo xtask build --arch x86-64 --config q35cpu` rc 0; `nm` on the emitted object shows `ap_trampoline_start` @ 0x0, `ap_trampoline_end` @ 0xe6
- **Committed in:** `b2ab7bb`

**4. [Rule 1 - Bug] `Mach` could not keep its `FromZeros` marker derive**
- **Found during:** Task 2 (dat.rs compile)
- **Issue:** `#[derive(FromZeros)]` on `Mach` requires every field to implement it; the plan-mandated `sched_context: SyncUnsafeCell<Label>` does not (nightly type, no zerocopy impl).
- **Fix:** Dropped the derive on `Mach` only (Label/PTable/Page/ExStack/KStack keep theirs).  No caller used the bound (no `Mach::new_zeroed` anywhere); both construction sites stand on their own SAFETY text (l.S zeroing + `Mach::init`; `alloc_zeroed` + zero-valid-by-construction fields + `init_ap_identity`), and the struct doc records the reasoning.
- **Files modified:** `x86_64/src/dat.rs`
- **Verification:** `cargo check -p x86_64 --target x86_64-unknown-none` rc 0
- **Committed in:** `b2ab7bb`

**5. [Rule 3 - Blocking] `global_asm!` template-string brace escaping**
- **Found during:** Task 2 (first build with the `global_asm!(include_str!(...))` mount)
- **Issue:** A comment's literal `{u16 limit, u32 base}` braces were parsed as Rust asm-template format placeholders (`expected }`), breaking the build.
- **Fix:** Reworded the comment without braces (the l.S convention — no braces anywhere in included asm).
- **Files modified:** `x86_64/src/ap_trampoline.S`
- **Verification:** full build rc 0
- **Committed in:** `b2ab7bb`

---

**Total deviations:** 5 auto-fixed (2 blocking, 1 missing-critical seam, 2 bugs)
**Impact on plan:** All five were necessary for correctness or compilability; no scope creep — the plan's must-haves are all present and verified.

## TDD Gate Compliance

Task 1 is `tdd="true"` in a `type: execute` plan (`workflow.tdd_mode` unset; the plan-level TDD gates do not apply).  The discipline was followed: RED commit `5aab161` (scaffolding + pins) precedes GREEN commit `d3a0b6c` (ancestry verified: GREEN descends from RED).

**Environment-qualified RED evidence:** the x86_64 crate's host-test harness cannot compile on this aarch64-apple-darwin host at all — sibling modules (`cpu.rs`, `trap.rs`, `swtch.rs`, `fpu.rs`, `pio.rs`) carry x86-only inline asm that is not cfg-gated for foreign host targets (PRE-EXISTING: the unmodified tree fails identically with 69 asm-class errors; zero reference the new test code).  The plan itself scopes these host tests to x86_64 hosts ("`cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu acpi` on x86_64 hosts"; xtask's arch-test leg is keyed to the host arch, so the x86_64 leg exists only there).  No Rosetta, no qemu-user-x86_64, and no x86_64-host std exist in the pinned nix toolchain.  The honest record (real command, real result, full analysis) is at `05-11-RED-task1.json`; it was deliberately NOT submitted to `gsd_run check tdd-red-evidence` because a build-failure-shaped record is INVALID_RED by construction and the classifier's gates are not active for this plan.  Locally the new test code is compile-proven by error-diffing `cargo check -p x86_64 --bins --target aarch64-apple-darwin --tests`; the assertion-level RED/GREEN run executes on CI x86_64 runners.  Recorded in `.planning/WINDOWS.md` as `unrun-verify` so the ship gate sees it.

## Issues Encountered

- **The trampoline/smp machinery is dead-stripped from the linked kernel until 05-12 calls it.**  `nm` showed the trampoline bytes and `ap_main` absent from the linked `x86_64-unknown-none-elf` kernel because nothing references the chain yet (lld GC of unreferenced sections; l.S's dead b1978 survives as part of the crate-root l.S emission, which initially obscured the mechanism).  Probed deliberately: anchoring `start_aps` (`black_box`) pulls the trampoline (at 0x20b000), `ap_main`, and its strings into the binary — confirming 05-12's real call sites anchor everything.  The plan's bar for this plan (assembles cleanly, compiles, clippy green) is met; binary residency arrives with 05-12's wiring, and this mechanism is recorded for it.
- **x86_64 boots cannot run on this Mac at all.**  The ISO build (`grub-mkrescue`) and the q35cpu image's authdomain generator are Linux-only in this toolchain (the boot-test attempt failed at image assembly with `R9_AUTHDOMAIN_GEN is not set`).  Per D-33 and the plan's own flagged assumption, the live `-smp 2` boot proof belongs to Plan 05-14 on CI TCG runners and the owner's Linux KVM host.  Local verification substituted: byte-level encoding proofs against the emitted object and the linked kernel, the full check/clippy/test/lintcontract chain green, and the dead-strip probe above.
- **Pre-existing audit drift found and recorded:** the UNSAFE-AUDIT x86_64 inventory's `main.rs` row read 9 blocks against a measured 10 at this plan's base (rows also summed to 156 against the header's stale 158).  The Task 1/2 deltas record the correction openly (main.rs row now 12; header now 178); no other rows were touched.

## Authentication Gates

None.

## User Setup Required

None — no external service configuration required.

## Known Stubs

- `x86_64/src/main.rs:413` — `ap_main` is the plan-declared temporary stub (marks the AP's Mach online, then parks).  Intentional: the plan's own artifact list and 05-12's Task 2 replace it in place (same file, same signature) with the full bring-up (GDT/TSS/IDT/LAPIC/SYSCALL MSRs/FPU/sched_loop).  It does not block this plan's goal (compile + clippy + assemble), and 05-14's boot gate proves the replacement.
- `x86_64/src/smp.rs:400` — `prepare_block()` is a deliberate no-op until 05-12's proc.rs wires `wake_pending` onto the per-CPU `Mach` fields this plan added (the port-side `prepare_block` call sites already exist; the arch hook joins with the proc migration).  Comment marks it.

## Threat Flags

None new.  The plan's threat register is implemented as written: T-05-19 (`mfence; lfence` before every x2APIC ICR write) lands in `lapic.rs::send_icr`; T-05-20 (the canonical bit-63 `intrspecial` test) lands in `trap.rs`; T-05-21 (strict `elen` bounds + counted MAX_CPUS cap) lands in `acpi.rs` with its pins; T-05-SC held (`Cargo.lock` untouched, zero installs).  No new network endpoints, auth paths, file-access patterns, or trust-boundary schema changes.

## Next Phase Readiness

- **Plan 05-12 (the x86_64 SMP-03 twin):** everything it needs is compiled and clippy-clean — the per-CPU `Mach` fields (its proc.rs migration target), `mach_ptr()`/`this_mach()`, the registered-hook surface (`register_port_smp`), `start_aps`, `lapic::init_ap`, and the `ap_main` stub it replaces in place.  Two of its planned call sites are this plan's anchor for the dead-stripped chain (see Issues).
- **Plan 05-14 (Owner Stop 2):** the live q35 `-smp 2` boot proof (CI TCG + owner KVM) exercises this plan's trampoline and handshake end to end; the evidence trail for it is the byte-level verifications recorded here plus the CI host-test legs.
- **Watch item for 05-12:** the BSP's `Mach.cpu_id`/`apic_id`/`online` initialization and `register_port_smp()` ordering belong to its main9 wiring (this plan deliberately leaves both unwired at runtime).

## Self-Check: PASSED

- Created files verified on disk with `[ -f ]`: `x86_64/src/ap_trampoline.S`, `x86_64/src/smp.rs`, `05-11-RED-task1.json`, and this SUMMARY.
- All 4 plan commits are ancestors of HEAD (`git merge-base --is-ancestor`): `5aab161` (T1 RED), `d3a0b6c` (T1 GREEN), `b2ab7bb` (T2), `70179f2` (T3).
- Measured commits from the ledger (`gsd-plan-head-before-05-11` = `d399118`): 4 at completion; `plan_head_after` = `70179f2`.
- Working tree clean at completion; `Cargo.lock` untouched (T-05-SC).
- Acceptance-criteria greps all pass (recorded per task above); the plan-level verification chain (`cargo xtask check`, `cargo xtask clippy --arch x86-64 --config q35cpu`, `cargo xtask test`, `cargo test -p xtask lintcontract`, full `cargo xtask build --arch x86-64 --config q35cpu`) all exit 0.

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*
