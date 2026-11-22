---
phase: 04-r9-userspace-exec-harvey-r9-abi
plan: 03
subsystem: kernel/abi
tags: [r9, aarch64, trap-frame, ureg, abi, cross-repo, build-time-check]
requires:
  - 04-01 (initrd delivery + D-14 measured syscall set)
  - 04-02 (paqfs reader + boot root-walk)
  - Phase 2 ABI freeze (aarch64/ABI.md, aarch64/include/ureg.h)
provides:
  - r9 TrapFrame ≡ Harvey Ureg field-for-field (35 × u64, stride 288)
  - D-06 layer 1: in-crate const asserts pinning every frame offset (always on)
  - D-06 layer 2: cargo xtask ureg-check cross-repo ureg.h gate (standalone + boot-test precondition)
  - aarch64/ABI.md "Kernel-side enforcement (Phase 4 appendix)" (D-07)
  - Unified 16-vector trap.S with per-origin SP capture + conditional SP_EL0 restore
affects:
  - 04-04 (exec frame seeding rides the pinned layout)
  - 04-06 (syscall result writeback via frame x0)
  - 04-07 (preemption saves/restores this frame)
  - 04-08 (note delivery rewrites frame pc/sp at the EL0 boundary)
actuals:
  tokens: 10200
  tasks: 3
  commits: 4
tech-stack:
  added: []
  patterns:
    - per-origin SP capture in one unified trap macro (EL0: MRS SP_EL0 post-switch; EL1: live entry-sp pre-switch with carrier-register park/recover)
    - conditional SP_EL0 restore on frame.psr & 0xF == 0 (9front vtrap1/noteret asymmetry)
    - two-layer cross-repo ABI gate (const asserts + xtask header parse with proven negative control)
key-files:
  created:
    - r9:xtask/src/uregcheck.rs
  modified:
    - r9:aarch64/src/trap.rs
    - r9:aarch64/src/trap.S
    - r9:aarch64/src/registers.rs
    - r9:xtask/src/main.rs
    - r9:HowItWorks.md
    - harvey:aarch64/ABI.md
key-decisions:
  - "Publication gate resolved: user selected 'publish' — the D-05/D-07 kernel-side ABI contract is published exactly as locked (frame layout + appendix contents incl. OQ-3 type-field informational encoding)"
  - "ESR_EL1/FAR_EL1 are never frame fields: read live at trap() entry, passed as dispatcher arguments; panic dump prints them from locals (x0..x30 row gate untouched)"
  - "EL1-origin x9 stays truthful: interrupted x9 parked on the interrupted kernel stack during entry-sp capture, recovered into its frame slot before the type store (deviation from plan-literal asm; see Deviations)"
patterns-established:
  - "ureg-check drift report = named-field expected-vs-found diff, non-zero exit; missing Harvey checkout skips bare builds but fails boot-test"
  - "boot-test preconditions run before qemu spawns (BootTestStep::run head)"
requirements-completed: [R9-04]
coverage:
  - id: AC-2-const-asserts
    description: "const asserts pin sp@248, psr@264, size 288 (D-06 layer 1)"
    requirement: R9-04
    verification:
      kind: automated
      ref: "grep of r9:aarch64/src/trap.rs + full cargo build (virt/default/raspi4b) green"
      status: pass
    human_judgment: false
  - id: AC-2-unified-frame
    description: "one unified frame, per-origin SP capture, conditional SP_EL0 restore; no EsrEl1 frame field"
    requirement: R9-04
    verification:
      kind: automated
      ref: "trap.S insns greps (mrs/msr sp_el0, spsr_el1, tst #0xf) + awk struct-region check + virtpanic/virt boot-tests"
      status: pass
    human_judgment: false
  - id: AC-2-no-regression
    description: "virtpanic (banner/start/31 rows/kernel halted) + virt (banner/3 ticks/echo) profiles and raspi builds stay green"
    requirement: R9-04
    verification:
      kind: automated
      ref: "cargo xtask boot-test --config virtpanic / --config virt; target/boot-test-{virtpanic,virt}.log"
      status: pass
    human_judgment: false
  - id: AC-3-appendix
    description: "ABI.md appendix published append-only after the D-17 section with pinned offsets, SP+8 formula, two-layer check"
    requirement: R9-04
    verification:
      kind: automated
      ref: "frozen-section greps + appendix-order awk + git diff (62 insertions, 0 deletions)"
      status: pass
    human_judgment: false
  - id: AC-3-ureg-check
    description: "ureg-check exits 0 on live tree; non-zero with named-field diff on drifted header (negative control)"
    requirement: R9-04
    verification:
      kind: automated
      ref: "cargo run -p xtask -- ureg-check; R9_HARVEY_DIR=<drifted> negative control names psr"
      status: pass
    human_judgment: false
  - id: AC-3-precondition-and-docs
    description: "ureg-check is a BootTestStep precondition; HowItWorks.md material-change entry + ABI.md pointer; xtask tests green"
    requirement: R9-04
    verification:
      kind: automated
      ref: "main.rs:874 call; grep 'ABI.md' HowItWorks.md; cargo test -p xtask (17 passed); virtpanic log shows ureg-check line before PASS"
      status: pass
    human_judgment: false
duration: "~8m (continuation segment; a prior segment reached the Task-1 publication checkpoint)"
completed: 2026-11-02
status: complete
---

# Phase 04 Plan 03: Trap-Frame ABI Publication + Static Enforcement Summary

r9's TrapFrame is now byte-identical in layout to Harvey's frozen `aarch64/include/ureg.h` (35 × u64, stride 288), pinned by always-on const asserts and a cross-repo `cargo xtask ureg-check` boot-test gate, with the kernel-side contract published as an appendix to `aarch64/ABI.md` — an ABI mismatch is a build failure, not a runtime mystery (SC#1).

## Performance

- **Duration:** ~8m continuation segment (Tasks 2–3 + verification + close-out); a prior segment reached the Task-1 decision checkpoint and was resolved by the user ("Select: publish").
- **Tasks:** 3/3 complete (Task 1 = checkpoint resolution, no code commit).
- **Commits:** 3 production + 1 metadata (r9: 177dc4c, 2303cf9; harvey: 90fa37493 + docs commit).
- **Verification runs:** 3 kernel builds (virt/default/raspi4b) + clippy, 2 boot-test profiles (virtpanic, virt) — each re-run with the ureg-check precondition active — ureg-check live + negative control, 17 xtask unit tests.

## Accomplishments

- **Task 1 (checkpoint resolution):** publication gate confirmed — the user selected **publish**; the D-05/D-07 artifacts (frame layout + appendix contents incl. the OQ-3 `type`-field informational encoding) are published exactly as locked.
- **Task 2 (r9 177dc4c):** TrapFrame reshaped to the frozen Ureg — tail swaps sources not offsets (`esr_el1`@248→`sp`, `elr_el1`@256→`pc`, `far_el1`@264→`psr`, `interrupt_type`@272 stays); ESR/FAR read live at `trap()` entry; 8 const asserts fail `cargo build` on drift; `trap.S` unified frame with per-origin SP capture (Pitfall 1) and conditional SP_EL0 restore on `frame.psr & 0xF == 0` (Pitfall 2); panic dump keeps the x0..x30 row shape (evidence: virtpanic log shows 31 rows, `sp`/`pc`/`psr` truthful — psr 0x600003c5 = EL1h+DAIF — `esr` decodes `Ok(Brk)` from the live register).
- **Task 3 (r9 2303cf9, harvey 90fa37493):** `xtask/src/uregcheck.rs` parses the frozen header (union = one slot, `r30` first-member name accepted), prints a named-field drift diff; `ureg-check` subcommand + BootTestStep precondition (missing Harvey checkout skips bare builds, fails boot-test); `aarch64/ABI.md` gains "Kernel-side enforcement (Phase 4 appendix)" append-only (62 insertions, 0 deletions); HowItWorks.md material-change entry + pointer.

## Task Commits

| Task | Repo | Commit | Files |
|------|------|--------|-------|
| 1 (checkpoint: decision = publish) | — | n/a (user decision, no code) | — |
| 2 | r9 | 177dc4c | aarch64/src/trap.rs, aarch64/src/trap.S, aarch64/src/registers.rs |
| 3 | r9 | 2303cf9 | xtask/src/uregcheck.rs (new), xtask/src/main.rs, HowItWorks.md |
| 3 | harvey | 90fa37493 | aarch64/ABI.md |

r9 HEAD after this plan: `2303cf93bd002de3f246ef94ada5ba8d6d364408` (D-13 evidence ordering: r9 commits landed before the Harvey-repo commit, which records them).

## Files Created/Modified

- **r9 `aarch64/src/trap.rs`** — TrapFrame ≡ Ureg + const asserts + live ESR/FAR in `trap()` + Debug tail renamed (x0..x30 rows byte-identical).
- **r9 `aarch64/src/trap.S`** — `handle_interrupt type, from_el1`: per-origin SP capture, pc/psr stores, conditional SP_EL0 restore; all 16 vector stubs carry their origin flag.
- **r9 `aarch64/src/registers.rs`** — `EsrEl1::read()` live-read helper (the plan named it; it did not exist).
- **r9 `xtask/src/uregcheck.rs`** (new) — D-06 layer-2 parser/gate + 5 unit tests.
- **r9 `xtask/src/main.rs`** — `ureg-check` subcommand + BootTestStep precondition.
- **r9 `HowItWorks.md`** — trap-frame material-change entry + ABI.md appendix pointer.
- **harvey `aarch64/ABI.md`** — "Kernel-side enforcement (Phase 4 appendix)" (the ONLY Harvey-repo code/doc change).

## Decisions Made

1. **Publish as locked** (Task-1 gate, user): frame layout 35 × u64 with sp@248/pc@256/psr@264/type@272, size 280/stride 288; appendix contents confirmed including OQ-3 (`type` = kernel-side informational vector index; the wire contract pins only the slot).
2. **ESR/FAR as dispatcher arguments** (D-05): the 04-04 fault handler's signature is `fault::user_fault(frame, esr, far)`; nothing reads them from the frame.
3. **Missing-Harvey policy for ureg-check:** standalone/bare builds print a skip line and continue (layer 1 guards); boot-test runs fail (the cross-repo gate is the point of the harness).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] EL1-origin save preserves the interrupted x9**
- **Found during:** Task 2 (trap.S implementation)
- **Issue:** the plan-literal `mov x9, sp` as the macro's first instruction clobbers the interrupted context's x9 before its frame slot is stored — the x9 slot would have held the entry-sp (falsifying the published-Ureg frame in EL1-origin dumps) and the restore pop would have resurrected the entry-sp into x9 on every EL1-trap return.
- **Fix:** keep the plan's carrier (`mov x9, sp` pre-switch capture) but park the interrupted x9 on the interrupted kernel stack first (16-byte carve), then recover it into its frame slot via x3 — before the type store, which reuses the parked bytes when the trap re-enters on the interrupt stack itself. x0's slot is unchanged (pre-existing interrupt-stack-top value; out of scope).
- **Files modified:** r9 `aarch64/src/trap.S`
- **Verification:** virtpanic dump shows a truthful x9 (`0x1`, not a stack pointer) and truthful `sp`/`pc`/`psr`; virt profile's repeated IRQ entry/return (3 ticks + echo) green.
- **Commit:** 177dc4c

**2. [Rule 1 - Bug] xtask compile errors in the new uregcheck wiring**
- **Found during:** Task 3 (first `cargo run -p xtask -- ureg-check`)
- **Issue:** dispatch referenced `uregcheck::UregCheckStep` (the struct lives in the crate root per the CleanStep idiom), an ambiguous `.into()` needed a `DynError` annotation, one unused import.
- **Fix:** three one-line compile fixes inline.
- **Files modified:** r9 `xtask/src/main.rs`, `xtask/src/uregcheck.rs`
- **Verification:** build + live check + negative control + 17 unit tests all green.
- **Commit:** 2303cf9

**Plan-supporting addition (in-plan intent):** `EsrEl1::read()` added to `registers.rs` (plan step 3 names the method; only the crate-level `ESR_EL1` register existed). Follows the file's `MidrEl1::read()` `cfg!(test)` idiom.

## Issues Encountered

None beyond the deviations above. All acceptance criteria passed on first verification run after the compile fixes; no verification-failure repair cycles needed.

## Known Stubs

None. The EC-0x15 SVC path intentionally keeps its print-and-park placeholder (real dispatch lands in 04-06 per the plan); no frame field, check, or doc is stubbed.

## Threat Flags

None. Both threat-model mitigations landed as designed: T-04-05 (ESR/FAR compile-enforced out of the frame — the `awk` struct-region check proves no `EsrEl1` field remains), T-04-06 (conditional SP_EL0 restore on EL0t; both return classes exercised by virtpanic EL1h-sync and virt IRQ profiles), T-04-07 (two layers, negative control proven, no warn-only path), T-04-SC (no package installs — uregcheck.rs is plain text parsing).

## User Setup Required

None.

## Next Phase Readiness

- 04-04 can seed exec frames against the pinned layout: the frame IS the Ureg, `frame.psr = 0` seeds EL0t, and the restore path already writes SP_EL0/ELR/SPSR from the frame on EL0-bound returns.
- The four-way pin is live: trap.S store slots ↔ trap.rs field offsets ↔ ureg.h field order ↔ const asserts ↔ xtask ureg-check — drift in any leg fails the build or the boot-test.
- Watch item for 04-04: `dispatch_irq` currently only recognizes EL1t/EL1h IRQ types; EL0-origin IRQs (9/13) fall into the dump-and-halt path until the scheduler tick work lands (04-07) — preemption planning must route those.
- Flagged assumption carried from the plan: R9-04's spec-less probe row remains a flagged assumption, not a resolved edge — acceptance here derived from ROADMAP SC#1 + CONTEXT D-05/D-06/D-07/D-08.

## Self-Check: PASSED

- Key files exist: r9 `aarch64/src/trap.rs`/`trap.S`/`registers.rs`, `xtask/src/uregcheck.rs`, `xtask/src/main.rs`, `HowItWorks.md`; harvey `aarch64/ABI.md` — all verified on disk.
- Commits exist: r9 `177dc4c`, `2303cf9` (`git log` confirms, on `main`); harvey `90fa37493` (on `master`). ≥1 `04-03` commit in BOTH repos ✓.
- Acceptance criteria re-run at close-out: const-assert greps, struct-region `EsrEl1` count 0, trap.S `mrs/msr sp_el0` + `mrs spsr_el1` + `tst #0xf` greps, ABI.md frozen-section/appendix-order/`sp@248`/`ureg->sp + 8` greps, append-only diff (62+/0-), ureg-check live + negative control, BootTestStep precondition grep, `cargo test -p xtask` (17 passed), virtpanic + virt boot-test profiles PASS with the precondition active.
- Plan-level verification: build-time layers fail on drift (asserts compiled in; ureg-check negative control), runtime profiles green, documents published.

---
*Phase: 04-r9-userspace-exec-harvey-r9-abi*
*Plan: 04-03*
