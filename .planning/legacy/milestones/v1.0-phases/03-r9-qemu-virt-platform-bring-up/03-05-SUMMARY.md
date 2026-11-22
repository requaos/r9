---
phase: 03-r9-qemu-virt-platform-bring-up
plan: "05"
subsystem: kernel
tags: [rust, aarch64, r9, qemu-virt, gicv2, generic-timer, pl011, irq, interrupt-dispatch, lock-free-echo, xtask, boot-test, acceptance-harness, nix-flake]

requires:
  - phase: 03-r9-qemu-virt-platform-bring-up (plan 02)
    provides: "trap init/unmask split (trap::unmask_irq is the P5 ordering point this plan's arm sequence converges on)"
  - phase: 03-r9-qemu-virt-platform-bring-up (plan 03)
    provides: "xtask [qemu] schema + accel/cpu override chain — BootTestStep consumes it via apply_to_qemu_step_with_overrides with zero schema edits"
  - phase: 03-r9-qemu-virt-platform-bring-up (plan 04)
    provides: "boot-proven virt tracer slice (banner on the canonical SC#2 machine), committed virt-gicv2.dtb, DTB-discovered polled-TX Pl011Uart + Console install"
provides:
  - "../r9/aarch64/src/platform/virt/gic.rs — GICv2 driver on the research Pattern 1 verified probe sequence: multi-compatible DTB discovery (arm,cortex-a15-gic or_else arm,gic-400), GICD/GICC mapping, enable(intid,prio) with ISENABLER/IPRIORITYR RMW + ITARGETSR cpu0 for SPIs only (P6), ack() via GICC_IAR, eoi() via GICC_EOIR, spurious-1023 never EOI'd"
  - "../r9/aarch64/src/platform/virt/timer.rs — ARM generic timer CNTP (PPI 14 -> INTID 30): runtime CNTFRQ_EL0 read (62.5 MHz TCG / 24 MHz HVF, never hardcoded), 1 Hz TVAL arm + per-tick re-arm, DTB INTID discovery with documented fallback (ROADMAP SC#3)"
  - "trap.rs IRQ dispatch branch — ack -> match 30/33/1023/default -> exactly one EOI except spurious -> RETURN (trap.S's eret resumes the wfi loop); raspi keeps the dump-and-spin path"
  - "PL011 RX IRQ path (SC#4): init_rx writes the verified Pattern 3 sequence (ICR=0x7ff, LCRH=0x70, IMSC=0x50, CR=0x301); rx_irq_echo drains on FR RXFE and echoes verbatim via raw DR writes — lock-free, never the Console lock (P8), bytes never interpreted"
  - "cargo xtask boot-test (D-11): zero-new-dependency unix-socket harness — capture-before-start via server=on,wait=on, ordered banner -> >=3 ticks -> typed-echo assertions, one overall budget (R9_BOOT_TEST_TIMEOUT), evidence log at target/boot-test-<config>.log"
  - "../r9/flake.nix apps.boot-virt — the acceptance gate as a flake app alongside apps.build"
affects: [03-06, phase-04-userspace-exec, phase-05-int-04]

# Actuals (#2632) — same scale as the plan's estimateTokens (chars/4 over realized diff).
# Cross-repo plan: commits = 3 r9 code commits + 1 harvey docs commit.
# r9 leg: c74fcb1 -> 1cc5127 (diff 36,671 chars -> 9,167 tokens). Harvey leg measured from ledger.
actuals:
  tokens: 9167
  tasks: 3
  commits: 4

# Commit ledger (#3968) — cross-repo plan: harvey base from the persisted ledger;
# the r9 leg is c74fcb1..1cc5127 (3 commits). commits: 4 = 3 r9 + 1 harvey docs commit.
plan_head_before: b59400e41bb30822bfedea0ed9d697d0662d51ec
plan_head_after: 421ec457f

tech-stack:
  added: []
  patterns:
    - "IRQ-context state discipline: every IRQ-reachable static (GIC, PL011 range) is a SyncUnsafeCell<Option<T>> published once before trap::unmask_irq and read lock-free thereafter — an mcslock taken in IRQ context can deadlock against its interrupted holder (P8 generalized beyond the Console)"
    - "GIC EOI discipline: spurious INTID 1023 is acked without EOI; every other INTID gets exactly one GICC_EOIR write after handling; unknown INTIDs log once and EOI cleanly instead of wedging (V7-in-spirit)"
    - "Zero-dep acceptance harness: qemu -chardev socket,server=on,wait=on + std::os::unix::net::UnixStream gives capture-before-start with no crates; assertion literals are named constants for 03-06 reuse; the echo assertion searches post-write bytes only (qemu socket serial does not self-echo)"

key-files:
  created:
    - "../r9/aarch64/src/platform/virt/gic.rs"
    - "../r9/aarch64/src/platform/virt/timer.rs"
  modified:
    - "../r9/aarch64/src/platform/virt/mod.rs"
    - "../r9/aarch64/src/platform/virt/pl011.rs"
    - "../r9/aarch64/src/trap.rs"
    - "../r9/aarch64/src/main.rs"
    - "../r9/xtask/src/main.rs"
    - "../r9/flake.nix"

key-decisions:
  - "GIC + PL011 handler statics use SyncUnsafeCell<Option<Gic/VirtRange>> published pre-unmask, NOT mailbox.rs's Lock<Option<T>> — the Lock idiom is wrong for IRQ-reachable state: the handler spinning on a lock held by the interrupted context never releases (handler runs to eret before the holder resumes); ack() on an uninitialised GIC degrades to the spurious INTID (acked, no EOI, return)"
  - "Tick prints go through the locked Console from IRQ context — safe ONLY because the post-unmask main9 loop is wfi and prints nothing; the invariant and its future-phase consequence (main-loop printing forces a lock-free tick path) are commented at the tick site per the plan's flagged assumption"
  - "boot-test asserts via an ordered marker state machine over one overall 300s budget (R9_BOOT_TEST_TIMEOUT) — never per-marker timings (P11); the echo probe 'r9echo7' is written only after >=3 ticks prove IRQs live, and searched for in bytes captured after the write"
  - "registers.rs needed NO change: UART0_DR/FR/LCRH/CR/IMSC/ICR all already existed (plan Task 2's confirm-then-add instruction resolved to confirm-only); the plan's files_modified list anticipated additions that weren't needed"
  - "boot-test dispatch runs BuildStep first in the same invocation (evidence freshness — never certifies a stale tree) and skips DistStep (virt's dist format is elf = no-op); the panic profile ([config] dev contains 'panic') currently asserts banner-only as plan-sanctioned scaffolding owned by 03-06"

patterns-established:
  - "Verified-probe-to-driver pipeline: gic.rs/timer.rs/pl011 init_rx are the research's empirically-verified register sequences transcribed with named constants and per-line comments — all three worked on first boot (14 ticks in 15s, echo on first probe)"
  - "Region-scoped P8 gate: awk '/pub fn rx_irq_echo/,/^}/' | grep -c println == 0 is part of the automated verify — the lock-free handler contract is source-enforced, not convention"

requirements-completed: [R9-01]
# R9-01 is shared by plans 03-02…03-06; the shared-ID gate keeps it open until the
# last declaring plan completes. This plan's copy of the ID is verbatim from the
# plan frontmatter; REQUIREMENTS.md stays Pending until 03-06 finishes.

coverage:
  - id: D1
    description: "SC#3 periodic ticks: GICv2 init + CNTP INTID 30 through the GIC, >= 3 '^tick' lines after the banner on the canonical machine, kernel returns from each IRQ to the wfi loop (14 ticks in 15s proves the eret path repeatedly); CNTFRQ read at runtime, TVAL re-armed per tick; unmask_irq last (P5)"
    requirement: R9-01
    verification:
      - kind: integration
        ref: "cd ../r9 && qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -smp 1 -m 1024M -accel tcg -display none -serial file:target/boot-logs/virt-ticks.log -kernel target/aarch64-unknown-none-elf/debug/aarch64 -dtb aarch64/lib/virt-gicv2.dtb → banner=1, ticks>=3, zero panic|Unhandled-interrupt markers (plan Task 1 gate)"
        status: pass
      - kind: integration
        ref: "15s soak: 14 ticks numbered 0..13 monotonic, zero 'unhandled IRQ' lines"
        status: pass
      - kind: other
        ref: "source gates: gic.rs carries arm,cortex-a15-gic + or_else + GICC_IAR + GICC_EOIR; timer.rs carries CNTFRQ_EL0 + CNTP_TVAL_EL0; main.rs carries unmask_irq; virt build green"
        status: pass
    human_judgment: false
  - id: D2
    description: "SC#4 interrupt-driven input: PL011 INTID 33 RX path arms IMSC=0x50/CR=0x301 after the GIC, rx_irq_echo drains on FR RXFE and echoes via raw DR writes — lock-free (region-scoped no-println proof), verbatim bytes only; ITARGETSR written for SPI 33, not for PPI 30 (P6)"
    requirement: R9-01
    verification:
      - kind: integration
        ref: "socket-chardev smoke (throwaway, uncommitted): banner -> ticks 0..2 -> typed 'r9echo7' echoed back through the INTID-33 path"
        status: pass
      - kind: integration
        ref: "cargo xtask boot-test --arch aarch64 --config virt exits 0; target/boot-test-virt.log shows banner, 3 ticks, 'r9echo7' in post-write bytes, zero wedge markers"
        status: pass
      - kind: other
        ref: "source gates incl. awk region-scoped no-println inside rx_irq_echo (P8); enable(33) present in main.rs before unmask_irq"
        status: pass
    human_judgment: false
  - id: D3
    description: "D-11 acceptance harness: boot-test builds the image it tests (BuildStep first), socket capture-before-start via server=on,wait=on, ordered banner/ticks/echo assertions, failure dumps the serial tail, evidence log per run; flake apps.boot-virt; zero new workspace deps; raspi3b/raspi4b builds and xtask unit tests green"
    requirement: R9-01
    verification:
      - kind: unit
        ref: "cargo test -p xtask → 9/9 pass (5 pre-existing config tests + 4 new marker-helper tests)"
        status: pass
      - kind: integration
        ref: "cargo xtask boot-test --arch aarch64 --config virt → PASS (exit 0, log written, 60146 bytes captured)"
        status: pass
      - kind: other
        ref: "nix flake show exposes apps.*.boot-virt; cargo xtask build --arch aarch64 [raspi3b] and --config raspi4b both Finished; git diff shows Cargo.toml/Cargo.lock untouched"
        status: pass
    human_judgment: false

# Metrics
duration: 10 min
completed: 2026-11-01
status: complete
---

# Phase 03 Plan 05: Interrupt bring-up + boot-test harness Summary

**Interrupts are live and proven on r9's virt platform: the GICv2 driver (research-corrected `arm,cortex-a15-gic` discovery), the ARM generic timer firing ~1 Hz ticks through INTID 30 (14 ticks in 15s under TCG, each returning through trap.S's eret to the wfi loop), and the PL011 RX path echoing typed input interrupt-driven and lock-free (INTID 33) — all wrapped in `cargo xtask boot-test`, a zero-new-dependency unix-socket harness that builds the image, boots with capture-before-start (`server=on,wait=on`), asserts banner → ≥3 ticks → typed-echo in order, writes the serial log to `target/boot-test-virt.log`, and exits 0; `flake.nix` exposes it as `apps.boot-virt`, raspi builds and all 9 xtask tests stay green, and Cargo.toml/Cargo.lock are untouched.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-11-01T23:48:59Z
- **Completed:** 2026-11-01T23:58:26Z
- **Tasks:** 3
- **Files modified:** 8 (all in sibling repo `../r9`; harvey side carries only this SUMMARY + state files per D-13)

## Accomplishments

- **ROADMAP Phase 3 SC#3 satisfied**: GICv2 initialized from the DTB (GICD 0x08000000 / GICC 0x08010000 mapped, CTLR/PMR/CTLR probe sequence), the CNTP generic timer (PPI 14 → INTID 30) fires on schedule — the captured log shows the banner then `tick 0..13` at ~1 Hz with zero wedge markers, and every tick returns to the wfi loop through trap.S's existing eret path.
- **ROADMAP Phase 3 SC#4 satisfied**: PL011 RX is interrupt-driven — `init_rx` arms IMSC=RXIM|RTIM and CR=UARTEN|TXE|RXE, INTID 33 (an SPI, so `gic::enable` writes its ITARGETSR cpu0 target — P6) dispatches to `rx_irq_echo`, which drains the FIFO and echoes bytes verbatim via raw DR writes. The typed probe `r9echo7` appears in the kernel log only through this path (qemu's socket serial does not self-echo).
- **P5 ordering contract honored**: `main9` virt tail is gic init → enable(30) → timer arm → pl011 init_rx → enable(33) → `trap::unmask_irq()` LAST → wfi loop. CNTFRQ_EL0 is read at runtime on every arm (62.5 MHz TCG / 24 MHz HVF, never hardcoded); TVAL is re-armed every tick.
- **IRQ safety discipline (P8 generalized)**: the UART echo handler is lock-free by construction (region-scoped awk gate proves no println inside it), GIC/handler statics are lock-free `SyncUnsafeCell` slots published pre-unmask, spurious INTID 1023 is acked without EOI, and unknown INTIDs log once + EOI cleanly rather than wedging.
- **D-11 harness landed**: `cargo xtask boot-test --arch aarch64 --config virt` — zero new crate dependencies (std::os::unix UnixStream + std::process only), builds the kernel it tests in the same invocation, captures from before guest start, asserts the ordered markers against one overall budget (never timings — P11), dumps the serial tail on failure, and writes the evidence log at a deterministic path. Assertion literals are named constants ready for 03-06's panic profile.

## Task Commits

Each task was committed atomically (repo: `../r9`, branch `main`):

1. **Task 1: gic.rs + timer.rs + trap IRQ dispatch — periodic ticks fire and return (SC#3)** — `032bdcc` (feat) — 5 files, +328/-3
2. **Task 2: PL011 RX IRQ path — interrupt-driven input with lock-free echo (SC#4)** — `4e35766` (feat) — 3 files, +66/-6
3. **Task 3: cargo xtask boot-test (D-11) + flake apps.boot-virt** — `1cc5127` (feat) — 2 files, +370/-1

**Plan metadata:** harvey docs commit (see below; `docs(03-05): complete interrupt bring-up + boot-test harness plan`)

## Files Created/Modified

- `../r9/aarch64/src/platform/virt/gic.rs` (created) — `Gic` driver: multi-compatible DTB discovery, GICD/GICC `map_device_register` mapping, probe-sequence init, `enable(intid,prio)` with SPI-only ITARGETSR, `ack()`/`eoi()`; lock-free static access (`init`/`enable`/`ack`/`eoi` free functions)
- `../r9/aarch64/src/platform/virt/timer.rs` (created) — `init(dt)` (DTB INTID discovery with documented 30 fallback, runtime CNTFRQ arm, CTL ENABLE), `tick()` (TVAL re-arm + monotonic `tick {n}` print + Console-lock forward note)
- `../r9/aarch64/src/platform/virt/mod.rs` (modified) — registers `gic` + `timer` modules
- `../r9/aarch64/src/platform/virt/pl011.rs` (modified) — gains `UART_INTID`, the `PL011_RANGE` handler static, `init_rx()` (verified Pattern 3 sequence), `rx_irq_echo()` (lock-free drain + verbatim echo)
- `../r9/aarch64/src/trap.rs` (modified) — `dispatch_irq` (virt): ack → match 30 tick / 33 echo / 1023 spurious-no-EOI / default log — EOI — RETURN; `not(virt)` stub returns false so raspi/host-test behavior is unchanged
- `../r9/aarch64/src/main.rs` (modified) — virt arm sequence (gic → timer → UART RX → unmask last) and the wfi idle loop; raspi's empty spin loop untouched under `not(virt)`
- `../r9/xtask/src/main.rs` (modified) — `boot-test` subcommand + `BootTestStep` (socket spawn, capture-before-start connect, ordered marker state machine, evidence log, tail-dump failures) + 4 unit tests for the pure marker helpers
- `../r9/flake.nix` (modified) — `apps.boot-virt` alongside `apps.build`/`apps.default`; devshell hint updated
- `../r9/aarch64/src/registers.rs` (**unchanged** — all six PL011 offset constants already existed; the plan's confirm-then-add instruction resolved to confirm-only)

(Harvey repo: only this SUMMARY + state files, per D-13.)

## Decisions Made

- **Lock-free statics for all IRQ-reachable state** — `SyncUnsafeCell<Option<Gic>>` / `SyncUnsafeCell<Option<VirtRange>>` published once before `unmask_irq`, read without locks in IRQ context. The mailbox `Lock<Option<T>>` idiom was explicitly rejected for this use: an MCS lock taken by the handler while its holder is interrupted can never release (the holder resumes only after eret). `ack()` on an uninitialized GIC degrades to the spurious INTID rather than reading uninit memory.
- **Tick println from IRQ context is safe-by-invariant** — post-unmask, `main9` sits in a wfi loop that prints nothing, so the Console mcslock is never held when a tick lands. The invariant and its consequence (any future main-loop print forces a lock-free tick path) are commented at the tick site, per the plan's flagged assumption.
- **boot-test assertion shape** — ordered state machine (banner → ticks → echo) over a single 300s budget (`R9_BOOT_TEST_TIMEOUT` override); the echo probe is written only after ticks prove IRQs live and is searched for only in bytes captured after the write. Failure output names the awaited marker and dumps the captured serial tail.
- **Panic profile scaffolding** — a `[config]` `dev` list containing `"panic"` selects the banner-only profile for now, exactly as the plan sanctions; 03-06 owns the real panic assertions.
- **boot-test mirrors the 03-03 override chain** — `--accel/--cpu` flags > `R9_ACCEL`/`R9_CPU` env > TOML > tcg/cortex-a72 built-ins on virt (never hvf — research P2), reusing `apply_to_qemu_step_with_overrides` with zero schema edits.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Tests] Unit tests for the boot-test marker helpers**
- **Found during:** Task 3 (harness implementation)
- **Issue:** the plan's verify requires `cargo test -p xtask` green but specified no coverage for the new assertion logic (`count_marker_lines`, `contains_marker`, the post-write echo window) — the harness's correctness gates the phase's SC evidence
- **Fix:** added a `#[cfg(test)] mod tests` in xtask/src/main.rs with 4 tests: CRLF line-initial counting, mid-line/missing marker exclusion, substring search, and the echo-window post-write-only property
- **Files modified:** ../r9/xtask/src/main.rs
- **Verification:** `cargo test -p xtask` → 9/9 pass (5 pre-existing + 4 new)
- **Committed in:** `1cc5127` (Task 3 commit)

### Process notes (not code deviations)

**2. Throwaway socket smoke test ahead of Task 3**
- **Found during:** Task 2 — the behavioral echo proof is delegated to Task 3's harness, but a wiring bug discovered after the harness encodes the assertion would cost a debug cycle
- **Handling:** a throwaway python3 socket-chardev script (in `target/boot-logs/`, gitignored, deleted after) confirmed banner → ticks → typed `r9echo7` echo on first try; the harness then reproduced the same mechanism in Rust. Nothing committed; no gate substitution
- **Committed in:** n/a

**3. registers.rs listed in Task 2 files but unchanged**
- The plan anticipated possibly adding PL011 offset constants; all six (DR/FR/LCRH/CR/IMSC/ICR) already existed at `registers.rs:15-22`. The confirm-then-add instruction resolved to confirm-only. Recorded so the file-list discrepancy is explicit rather than silent.

---

**Total deviations:** 1 auto-fixed (Rule 2) + 2 process notes
**Impact on plan:** No scope creep and no behavioral drift — the Rule-2 addition only tests code the plan mandated; every driver line is the research-verified probe sequence; all must_have truths, prohibitions (no Console lock in IRQ context, no ITARGETSR for PPIs, verbatim echo, zero new crates, no timing assertions) hold and are source- or boot-proven.

## Issues Encountered

None. All three drivers booted clean on first attempt — the research's empirically-verified register sequences (Pattern 1/2/3) and 03-04's boot-proven tracer slice did exactly the de-risking they were designed for. Every written verify gate passed unsubstituted on its first run.

## User Setup Required

None - no external service configuration required. (D-10's linux-host verification legs remain queued for end-of-phase UAT from 03-01; this plan adds no new ones.)

## Next Phase Readiness

- **03-06 builds directly on this plan's seams**: the panic config flows through the existing `[config] dev = ["panic"]` mechanism (check-cfg predeclared in aarch64/Cargo.toml), `BootTestStep` already selects the panic profile (banner-only scaffolding today — swap in the register-dump assertions), the sync/syscall trap branches keep their dump-and-spin shape as the panic-path foundation, and the assertion literals are named constants (`BOOT_BANNER_MARKER`/`BOOT_TICK_MARKER`/`BOOT_ECHO_PROBE`) ready for reuse.
- **trap return path proven repeatedly** (14 IRQ round-trips in one soak) — the eret discipline Phase 4's EL0/`svc` work depends on is now load-bearing and green.
- **Forward invariant recorded in code**: the tick print's IRQ-context safety depends on the wfi main loop printing nothing; if 03-06's panic config or a later phase adds main-loop printing, move the tick print to a lock-free path first (comment at `timer.rs` tick site).
- R9-01 remains open (shared by plans 03-02…03-06; the shared-ID gate marks it when the last declaring plan completes).

---
*Phase: 03-r9-qemu-virt-platform-bring-up*
*Completed: 2026-11-01*

## Self-Check: PASSED

- FOUND: ../r9/aarch64/src/platform/virt/gic.rs
- FOUND: ../r9/aarch64/src/platform/virt/timer.rs
- FOUND: ../r9/aarch64/src/platform/virt/pl011.rs (init_rx + rx_irq_echo)
- FOUND: ../r9/aarch64/src/trap.rs (IRQ dispatch branch)
- FOUND: ../r9/aarch64/src/main.rs (virt arm sequence + wfi loop)
- FOUND: ../r9/xtask/src/main.rs (boot-test subcommand + BootTestStep)
- FOUND: ../r9/flake.nix (apps.boot-virt)
- FOUND: r9 commit 032bdcc (`feat(03-05): GICv2 + generic timer drivers, trap IRQ dispatch with eret return (SC#3)`, branch main)
- FOUND: r9 commit 4e35766 (`feat(03-05): PL011 RX IRQ path with lock-free echo (SC#4)`, branch main)
- FOUND: r9 commit 1cc5127 (`feat(03-05): cargo xtask boot-test — zero-dep socket harness asserting banner/ticks/echo (D-11)`, branch main)
- FOUND: .planning/phases/03-r9-qemu-virt-platform-bring-up/03-05-SUMMARY.md
- Cargo.toml/Cargo.lock diff vs r9 base c74fcb1: empty (zero new dependencies per prohibition)
