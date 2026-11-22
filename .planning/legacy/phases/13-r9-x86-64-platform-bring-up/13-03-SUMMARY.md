---
phase: 13-r9-x86-64-platform-bring-up
plan: 03
subsystem: kernel
tags: [x86_64, q35, lapic, x2apic, xapic, tsc-deadline, periodic-timer, idt, trap-dispatch, polled-rx, boot-test, kvm, tcg]

# Dependency graph
requires:
  - phase: 13-r9-x86-64-platform-bring-up (13-02)
    provides: the main9-x64 arc (platform/q35 slot, mb2 facts, masked-IRQ park to replace with the arming window), the loaded IDT/GDT/TSS, the identity-mapped execution model, and the gate's ordered-marker machinery
provides:
  - The ticking, listening q35 kernel: CPUID-gated dual-mode LAPIC registry (x2APIC MSR map primary, classic xAPIC MMIO fallback), the CPUID-gated TSC-deadline(0x6E0)/periodic tick source with runtime-resolved frequency, the trap.rs IRQ dispatch (EOI strictly after device work), the polled 16550 RX echo, and main9's arming window (sti LAST)
  - The Phase-3-style platform gate green twice consecutively under KVM-native (cpu=host) AND green under the TCG fallback (--accel tcg --cpu qemu64) — X64-01 closed end-to-end
  - The root-cause finding every later x86_64 plan inherits: MSR 0x832 is the x2APIC-mode LVT-timer address (KVM gates 0x800-0xBFF on x2APIC mode); IA32_TSC_DEADLINE is 0x6E0, valid in either APIC mode
  - The LAPIC/timer half of X64-02's substrate (IOAPIC/INTx arrive with 13-05)
affects: [13-04, 13-05, 13-06, "Phase 14 XEXEC (the tick source deadlines consume)"]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
# Same estimateTokens scale (chars/4 over the realized diff), never a harness token count.
actuals:
  tokens: 16160   # chars/4 over git diff b964a62..351287f in ../r9 (64640 chars, 11 files; plan estimated 62000, confidence low — the overestimate direction; three dispatches of host-chase diagnostics live on the harvey side instead)
  tasks: 3
  commits: 4      # MEASURED harvey-repo commits per the plan ledger (05b709524..docs commit: three 13-03 diagnostics commits + this docs commit). Production code: 5 commits in the sibling r9 repo (c39a210, 0877f3a, 333b940, 607a7c5, 351287f).
plan_head_before: 05b709524ec36508474ab201cff71c89a14e64cd
plan_head_after: 987414f1a4902a41ee5d0f0a424b4e15ac1c6370   # harvey-side pre-docs HEAD; all production commits live in ../r9 (b964a62..351287f)

# Tech tracking
tech-stack:
  added: []        # zero new crates — the from-scratch discipline holds
  patterns:
    - "Dual-mode device registry: the same publish-once SyncUnsafeCell contract with mode-dispatched accessors (MMIO vs MSR map), the mode CPUID-gated at discovery — hardware capability selects the access path, never a config flag"
    - "Capability-fact discovery print: the mode gates print the raw CPUID bits they keyed off (x2apic=true tsc-deadline=true), so every gate leg's log carries the evidence the mode selection ran on"
    - "Strip-debug ISO staging: GRUB loads only PT_LOADs, so the xtask stages an objcopy --strip-debug kernel into the ISO (4.4 MB vs 18 MB; the mb2 header check re-runs on the stripped bytes)"
    - "Accel-aware wall-clock budgets: TCG multiplies the first-marker and overall budgets — SeaBIOS+GRUB+BIOS-disk load dominates and would false-red the gate"

key-files:
  created:
    - ../r9/x86_64/src/platform/q35/lapic.rs
    - ../r9/x86_64/src/platform/q35/timer.rs
  modified:
    - ../r9/x86_64/src/cpu.rs
    - ../r9/x86_64/src/pio.rs
    - ../r9/x86_64/src/trap.rs
    - ../r9/x86_64/src/uart16550.rs
    - ../r9/x86_64/src/main.rs
    - ../r9/x86_64/src/l.S
    - ../r9/xtask/src/main.rs

key-decisions:
  - "The KVM #GP root cause (user-supplied via dispatch 3, confirmed and sharpened): MSR 0x832 is the x2APIC-mode address of the LVT-timer register itself (0x800+(0x320>>4), SDM 10.12.2) and KVM gates MSR range 0x800-0xBFF on the guest APIC actually being in x2APIC mode — wrmsr(0x832) from xAPIC mode is an architectural #GP on ANY host, exactly matching the retest matrix (apicv-independent, kernel-independent, any bit-24 CPU model). IA32_TSC_DEADLINE is 0x6E0 (SDM Vol 3A 10.5.4.2; Linux/qemu MSR_IA32_TSC_DEADLINE), valid in EITHER APIC mode"
  - "The r9 LAPIC runs CPUID.01H:ECX[21]-gated x2APIC when the CPU advertises it (KVM cpu=host passes the i7-1260P's x2APIC through) — SVR enable first (the emulator-proven order), then IA32_APIC_BASE EXTD (bit 10) with ENABLE (bit 11) ensured in the same write, then LVT programming over the MSR map; classic xAPIC MMIO stays the fallback for hardware without bit 21 (TCG's qemu64 leg keeps exercising it — the gate proves the CPUID gating both ways)"
  - "The plan's own must_have encoded the bug: 'timer.rs contains 0x832' was a researcher mis-mapping of the TSC-deadline MSR; the corrected constant 0x6E0 keeps every other design element (CPUID gating, one-shot re-arm-in-tick, runtime frequency) unchanged"
  - "TSC frequency resolves at runtime with printed provenance (CPUID 0x15 ratio -> CPUID 0x16 nominal -> PIT channel-2 calibration), sanity-banded: synthetic qemu CPU models report nonsense leaf values (Skylake-Client under TCG claims a ~68 kHz crystal) and out-of-band values fall through to the next source"
  - "The 16550 RX path is LSR-polled from the timer tick sweep (the bring-up shape); qemu's socket serial never self-echoes, so the r9echo7 probe reappearing on the wire proves the bytes entered the kernel"

patterns-established:
  - "x2APIC dual-mode access: one offset-space API, accessors dispatch MMIO vs MSR by the registry's recorded mode — later x86_64 devices (IOAPIC is MMIO-only) copy the publish-once + tolerant-degrade shape, not the MSR map"
  - "Dump-once-and-park exception handling: a synchronous exception at boot prints one Ureg frame and halts — returning re-executes the faulting instruction and one #GP becomes a 6 MiB serial storm (observed live)"

requirements-completed: [X64-01]

# Coverage metadata (#1602) — one entry per shipped deliverable.
coverage:
  - id: D1
    description: "cpu.rs/pio.rs substrate: cpuid (rbx-preserved), rdtsc, the pure has_tsc_deadline/has_x2apic leaf-1 ECX decision helpers, and the pio inb/inw/inl port-read family"
    requirement: X64-02
    verification:
      - kind: unit
        ref: "r9 host tests: timer::tests::mode_gate_keys_off_cpuid_leaf1_ecx_bit24, lapic::tests::apic_mode_keys_off_cpuid_leaf1_ecx_bit21 (both route through the cpu.rs helpers)"
        status: pass
      - kind: other
        ref: "acceptance greps: 'fn cpuid'==1 in cpu.rs, 'fn inb'==1 in pio.rs"
        status: pass
    human_judgment: false
  - id: D2
    description: "lapic.rs publish-once registry, dual-mode: x2APIC MSR map behind CPUID bit 21 (EXTD after the SVR enable; LINT0/1 masked per D-04; error LVT; drain EOI) with classic xAPIC MMIO fallback; lock-free eoi() and timer-LVT/DCR/ICR/CCR surfaces"
    requirement: X64-02
    verification:
      - kind: unit
        ref: "r9 host tests: lapic::tests::apic_mode_keys_off_cpuid_leaf1_ecx_bit21, lapic::tests::x2apic_register_map_arithmetic (10/10 total green)"
        status: pass
      - kind: integration
        ref: "live serial both ways: KVM cpu=host prints 'lapic: x2apic live (msr 0x800 map; lint0/1 masked, no pic)'; TCG qemu64 prints 'lapic: xapic 0xfee00000 live (lint0/1 masked, no pic)'"
        status: pass
    human_judgment: false
  - id: D3
    description: "timer.rs CPUID-gated tick source: TSC-deadline (arm = wrmsr(0x6E0) = rdtsc()+delta, one-shot, re-armed inside tick) vs LAPIC periodic (DCR divide + ICR, LAPIC-rate calibrated against the resolved TSC), one tick()/ticks() surface, runtime-resolved frequency with printed provenance"
    requirement: X64-02
    verification:
      - kind: unit
        ref: "r9 host tests: mode_gate_keys_off_cpuid_leaf1_ecx_bit24, tsc_delta_converts_without_overflow_at_q35_scale, tick_accounts_once_and_rearms"
        status: pass
      - kind: integration
        ref: "live serial: 'timer: 2496092880 Hz (pit 1193182 Hz reference), TscDeadline mode, armed' (KVM) / 'Periodic mode, armed' (TCG)"
        status: pass
    human_judgment: false
  - id: D4
    description: "trap.rs IRQ dispatch: vectors >= 32 dispatch in the aarch64 shape — handler FIRST, then lapic::eoi() (Pitfall 5 ordering); strays print 'unhandled IRQ {n}' and still EOI; the spurious vector skips EOI per spec; exceptions dump once and park"
    requirement: X64-02
    verification:
      - kind: e2e
        ref: "the gate's >= 3 tick lines + the r9echo7 echo ride the dispatch end-to-end (timer vector handled, EOI'd, re-armed) on both accels"
        status: pass
    human_judgment: false
  - id: D5
    description: "main9 arming window + polled echo: lapic::init -> timer::init -> sti LAST (the Phase-3 hazard comment pinned), then the hlt idle loop; uart16550 getb/rx_echo wired into the tick sweep"
    requirement: X64-01
    verification:
      - kind: e2e
        ref: "grep 'cpu::sti' main.rs == 1 adjacent to the unmask-LAST comment; the typed r9echo7 probe reappears on the socket through the kernel echo path (grep -c r9echo7 target/boot-test-q35.log == 1 on the final KVM run)"
        status: pass
    human_judgment: false
  - id: D6
    description: "xtask q35 gate grown to the Ticks profile: banner -> >= 1 q35: memory fact -> >= 3 tick lines -> typed echo probe, order-anchored via marker_position, negative panic scan, accel-aware budgets, strip-debug ISO staging with the mb2 header check re-run on the stripped bytes"
    requirement: X64-01
    verification:
      - kind: unit
        ref: "r9 xtask tests incl. q35_memory_facts_count_only_after_the_banner, x64_tick_lines_count_only_past_the_facts (84 green)"
        status: pass
      - kind: e2e
        ref: "boot-test PASS lines naming the full chain on every leg of the final matrix"
        status: pass
    human_judgment: false
  - id: D7
    description: "The Phase-3-style platform gate green on repeated runs: KVM-native default twice consecutively (x2apic + TSC-deadline) AND the TCG fallback leg (xapic + periodic) — X64-01's completion clause"
    requirement: X64-01
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch x86-64 --config q35 x2 green (plus a third evidence run); --accel tcg --cpu qemu64 green; aarch64 virt regression, check, clippy, 10/10 host tests green"
        status: pass
    human_judgment: false

# Metrics
duration: 12h 41m
completed: 2026-11-07
status: complete
---

# Phase 13 Plan 03: LAPIC + Timer + Dispatch — the Ticks/Echo Gate Summary

**The q35 kernel ticks and listens: a CPUID-gated dual-mode LAPIC (x2APIC MSR map on KVM cpu=host, xAPIC MMIO fallback under TCG), the TSC-deadline(0x6E0)/periodic tick with runtime-resolved frequency, IDT dispatch with EOI-after-work, and the polled r9echo7 echo — the platform gate green twice consecutively on KVM and green on TCG, after root-causing the KVM #GP as the x2APIC MSR-range gate meeting a wrong deadline constant (0x832 is the x2APIC LVT-timer address, not IA32_TSC_DEADLINE).**

## Performance

- **Duration:** 12h 41m wall (three dispatches: initial execution, two host-chase dispatches, and the dispatch-3 fix; includes the blocked-on-host-fix and user-decision windows between checkpoints)
- **Started:** 2026-11-07T01:19:47Z
- **Completed:** 2026-11-07T14:05:00Z
- **Tasks:** 3 (Task 1 auto, Task 2 auto/tdd, Task 3 tracer)
- **Files modified:** 11 in ../r9 (2 created, 9 modified) + this plan's harvey docs

## Accomplishments
- X64-01 is closed end-to-end: banner -> memory facts -> >= 3 ticks -> the typed r9echo7 echo, order-anchored, timing-tolerant, negative panic scan on — green twice consecutively under the KVM-native default and green under the documented TCG fallback (`--accel tcg --cpu qemu64`)
- The LAPIC registry runs dual-mode in the gic/ publish-once contract: CPUID bit 21 selects x2APIC (EXTD switch after the standard SVR enable; LINT0/1 masked per D-04; MSR accessors over the 0x800+(offset>>4) map); hardware without the bit keeps classic xAPIC MMIO — the same lock-free eoi()/LVT surfaces either way
- The KVM #GP root cause is pinned and fixed: MSR 0x832 is x2APIC-gated space (the LVT-timer's x2APIC address), so the xAPIC-mode guest's "deadline arm" was an architectural fault on any host; the deadline now arms at 0x6E0 (valid in either mode) — the user-supplied hypothesis, confirmed by a one-constant probe run and then by the full matrix
- The timer resolves its frequency at runtime with printed provenance (CPUID 0x15 -> 0x16 -> PIT channel-2 one-shot; synthetic-model leaf values sanity-banded) and calibrates the periodic LAPIC rate against the resolved TSC — never a compiled-in Hz
- The trap dispatch carries the Pitfall-5 ordering (device work, THEN EOI), strays print and still EOI, the spurious vector skips EOI per spec, and a boot exception dumps once and parks instead of storming serial

## Task Commits

Each task was committed atomically (r9 repo, `main`):

1. **Task 1: cpu.rs/pio.rs substrate + lapic.rs — the xAPIC publish-once registry** - `c39a210` (feat)
2. **Task 2 RED: failing timer mode/delta/tick host tests** - `0877f3a` (test)
3. **Task 2 GREEN: CPUID-gated TSC-deadline/periodic timer + trap IRQ dispatch** - `333b940` (feat)
4. **Task 3: main9 arming window + polled console echo — the q35 gate green twice on TCG** - `607a7c5` (feat)
5. **Dispatch-3 fix: x2APIC mode + the true IA32_TSC_DEADLINE address 0x6E0 — the KVM gate green** - `351287f` (fix)

**Harvey-side diagnostics committed during the blocked window** (the two-dispatch host chase; evidence, not code): `4480b5ab8` (blocker diagnostics + TDD RED evidence), `1cfca5a06` (post-host-fix retest: defect narrowed to the wrmsr(0x832) refusal), `987414f1a` (option-A retest: apiv=Y proven irrelevant).

**Plan metadata:** (this harvey docs commit — see below)

## TDD Gate Compliance

Task 2 executed RED -> GREEN -> REFACTOR per the plan's `tdd="true"` contract:
- **RED:** all three behavior tests failed on their planned assertions against loud stubs (`timer::tests::mode_gate_keys_off_cpuid_leaf1_ecx_bit24` and companions); evidence record at `13-03-red-evidence.json` in this directory. Rust's libtest console has no classifier adapter and `workflow.tdd_mode` is false in config — the record is on disk for review (the documented Rust gap, tdd.md #4379)
- **GREEN:** commit `333b940` — `cargo test -p x86_64 timer` 3 passed
- **REFACTOR:** none needed; the dispatch-3 fix added two further host tests (the apic mode gate + the x2APIC register-map arithmetic) alongside its implementation, keeping the suite green rather than staging a synthetic RED (10/10 total)

## Files Created/Modified
- `../r9/x86_64/src/platform/q35/lapic.rs` - (new) the dual-mode publish-once registry: named xAPIC offsets, ApicMode + the pure apic_mode gate, MMIO/MSR dispatching accessors with bounds checks, init (SVR enable first -> EXTD switch when bit 21 -> LVT programming -> drain EOI -> publish -> mode print), lock-free eoi()/timer surfaces; 2 host tests
- `../r9/x86_64/src/platform/q35/timer.rs` - (new) the CPUID-gated tick source: TimerMode/timer_mode, tsc_delta, tick_plan (re-arm FIRST, account once), runtime frequency chain (0x15/0x16 sanity-banded -> PIT), periodic LAPIC-rate calibration, the 0x6E0 deadline arm; 3 host tests
- `../r9/x86_64/src/cpu.rs` - cpuid (rbx preserved through scratch operands), rdtsc, has_tsc_deadline + has_x2apic pure helpers, wrmsr doc re-keyed to 0x6E0
- `../r9/x86_64/src/pio.rs` - inb/inw/inl mirroring the out* family
- `../r9/x86_64/src/trap.rs` - the dispatch rebuild: vectors >= 32 -> dispatch_irq (handler, then eoi; strays print+EOI; spurious skips EOI); exceptions dump once and park
- `../r9/x86_64/src/uart16550.rs` - getb (LSR-ready + RBR pop) and the bounded rx_echo sweep
- `../r9/x86_64/src/main.rs` - the arming window (lapic::init -> timer::init -> sti LAST) + the hlt idle loop
- `../r9/x86_64/src/l.S` - warp64 sets the kernel %gs base via IA32_GS_BASE wrmsr (no FSGSBASE under TCG's qemu64); the UC-APIC-page experiment reverted with a do-not-resplit comment
- `../r9/xtask/src/main.rs` - the Ticks-profile gate (order-anchored banner/facts/ticks/echo), accel-aware budgets, strip-debug ISO staging, X64 PASS message; new unit tests

## Decisions Made
- The dispatch-3 fix shape: BOTH halves were needed — entering x2APIC mode makes the register file ride the MSR map (and is the modern mode the host itself runs), while the corrected 0x6E0 deadline address is what actually un-gates the timer (valid in either APIC mode; a 0x832 write from x2APIC mode would clobber the LVT entry, never arm a deadline)
- The x2APIC switch sits AFTER the SVR software-enable (the emulator-proven enable-first order stays) and sets ENABLE|EXTD together in one IA32_APIC_BASE write (SDM 10.12.2: the unit must be enabled for the EXTD transition; firmware's bit 11 is preserved-OR'd rather than assumed)
- The gate CPU stays D-01's pinned `cpu = "host"` — option B (pinning a bit-24-clear CPU) was never needed; the KVM-native leg now exercises the primary x2apic + TSC-deadline mode as designed
- Frequency provenance is printed on every boot (source + Hz) so a mis-calibration is visible in any gate log, not discovered downstream

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The TSC-deadline MSR constant was wrong: 0x832 is x2APIC-gated LVT-timer space, not IA32_TSC_DEADLINE**
- **Found during:** Task 3's KVM gate leg (the #GP(0) on wrmsr(0x832) at pc 0x203ffd); root cause supplied by the user at dispatch 3, confirmed by a one-constant probe run, then the full matrix
- **Issue:** The plan's must_have ("timer.rs contains 0x832") and RESEARCH text encoded a mis-mapped MSR: 0x832 is the x2APIC-mode address of the LVT-timer register (0x800+(0x320>>4)), and KVM gates MSR range 0x800-0xBFF on the guest APIC being in x2APIC mode — from the kernel's xAPIC MMIO mode the write took #GP(0) on ANY host (matching the entire retest matrix: apicv Y/N, kernels 7.3.0-rc6/7.3.1, host/Skylake CPU models, periodic always green). IA32_TSC_DEADLINE is 0x6E0
- **Fix:** `MSR_IA32_TSC_DEADLINE = 0x6E0` (valid in either APIC mode), with the root-cause story pinned in the constant's comment; every stale 0x832 reference in the file docs re-keyed
- **Files modified:** ../r9/x86_64/src/platform/q35/timer.rs, ../r9/x86_64/src/cpu.rs
- **Verification:** probe run (0x6E0 alone, still xAPIC mode) green under KVM with TscDeadline ticks — the corrected address arms the timer from either mode; then the full matrix
- **Committed in:** 351287f

**2. [Rule 3 - Blocking] x2APIC mode implemented (the plan's "no x2APIC this phase" prohibition overridden by the dispatch)**
- **Found during:** dispatch 3 (the user-directed fix: "enable x2APIC mode and use MSR accessors, CPUID-gated, keeping the xAPIC path as fallback")
- **Issue:** The kernel ran xAPIC MMIO mode while the gate host's KVM exposes x2APIC — the modern register path was unused and the 0x832 MSR-range confusion lived exactly in that gap
- **Fix:** CPUID.01H:ECX[21]-gated x2APIC in the same publish-once registry: SVR enable first, IA32_APIC_BASE EXTD (bit 10, ENABLE bit 11 ensured) in one write, LVT programming over MSR accessors (0x800+(offset>>4)), `lapic: x2apic live` vs `lapic: xapic ... live` in the discovery print, the capability facts printed (`x2apic=true tsc-deadline=true`)
- **Files modified:** ../r9/x86_64/src/platform/q35/lapic.rs, ../r9/x86_64/src/cpu.rs (+2 host tests)
- **Verification:** KVM legs print `x2apic live` + TscDeadline and pass twice consecutively; TCG qemu64 prints `xapic 0xfee00000 live` + Periodic and passes — the CPUID gating proven both ways
- **Committed in:** 351287f

**3. [Rule 1 - Bug] warp64's CR4.FSGSBASE enable wedged early boot under TCG (607a7c5)**
- **Issue:** TCG's qemu64 has no FSGSBASE; the ungated CR4 bit pinned EIP on the mov-to-cr4 (found by live EIP sampling)
- **Fix:** CR4.FSGSBASE is never enabled; warp64 sets the kernel %gs base via the IA32_GS_BASE MSR — valid everywhere
- **Files modified:** ../r9/x86_64/src/l.S — **Committed in:** 607a7c5

**4. [Rule 3 - Blocking] GRUB took 20+ min to load the 18 MB debug image under TCG (607a7c5)**
- **Fix:** the ISO stages an objcopy --strip-debug kernel (4.4 MB, ~3 min load; the mb2 header check re-runs on the stripped bytes); accel-aware gate budgets (TCG multiplies first-marker/overall)
- **Files modified:** ../r9/xtask/src/main.rs — **Committed in:** 607a7c5

**5. [Rule 1 - Bug] Emulator APICs discard LVT writes while software-disabled (607a7c5)**
- **Fix:** the SVR software-enable moved FIRST (the 9k apiconline order — observed live: LVT programming silently vanished before it)
- **Files modified:** ../r9/x86_64/src/platform/q35/lapic.rs — **Committed in:** 607a7c5

**6. [Rule 1 - Bug] One boot #GP became a 6 MiB serial storm (607a7c5)**
- **Fix:** synchronous exceptions dump one Ureg frame and park (`kernel halted`) instead of returning into the faulting instruction
- **Files modified:** ../r9/x86_64/src/trap.rs — **Committed in:** 607a7c5

**7. [Rule 3 - Blocking] Synthetic qemu CPU models report nonsense CPUID 0x15/0x16 frequencies (607a7c5)**
- **Fix:** sanity-banded leaf values (a ~68 kHz "crystal" measured live) fall through to PIT calibration; provenance printed
- **Files modified:** ../r9/x86_64/src/platform/q35/timer.rs — **Committed in:** 607a7c5

---

**Total deviations:** 7 auto-fixed (3 Rule 1 bugs, 4 Rule 3 blockers; #1-#2 are the dispatch-3 continuation fix, #3-#7 were fixed inside the original task commits)
**Impact on plan:** All fixes protect the plan's own gates; the dispatch-3 pair (root cause + x2APIC mode) closes the KVM leg without touching D-01's pinned CPU or any qemu flag — the kernel-side-only resolution the user directed. No scope creep.

## Issues Encountered
- The KVM gate leg was chased across two host-side dispatches before the guest-side root cause landed: (a) kernel 7.3.0-rc6's APICv MMIO-write loss was REAL (KVM's canonical LAPIC state never saw guest writes) and was fixed by the user's 7.3.1 bump — but it was a distinct defect that masked ours; (b) the enable_apicv coupling was hypothesized and disproven (identical #GP with apicv=N and Y on 7.3.1). The persisting #GP on wrmsr(0x832) for any bit-24 CPU model — with periodic mode green under KVM — was the tell that finally pointed at MSR-range gating rather than LAPIC state sync. Full evidence: `13-03-kvm-evidence-20261107.txt` (RETEST 1-3); the deferred-items entry is resolved with the true root cause
- The 4-KiB-UC APIC-page split experiment (SDM-sanctioned mapping hygiene) silently broke APIC MMIO under BOTH accels and was reverted with a do-not-resplit comment in l.S — the plain 1 GiB WB boot window delivers

## Verification Log

Final matrix on the closing tree (r9 `351287f`):
- **KVM-default gate x2 (consecutive) + a third evidence run** — PASS: `lapic: cpuid leaf-1 ecx: x2apic=true tsc-deadline=true`, `lapic: x2apic live (msr 0x800 map; lint0/1 masked, no pic)`, `timer: 2496092880 Hz (pit 1193182 Hz reference), TscDeadline mode, armed`, banner -> 9 memory facts + acpi rsdt -> tick 0/1/2 -> r9echo7 (grep count 1 in the final KVM log)
- **TCG fallback** (`--accel tcg --cpu qemu64`) — PASS: `x2apic=false tsc-deadline=false`, `lapic: xapic 0xfee00000 live`, `Periodic mode, armed`, full marker chain — the CPUID gating proven both ways
- **aarch64 virt regression** — PASS (banner + >= 3 ticks + echo)
- **Host tests** — 10/10 (8 prior + the 2 new lapic tests); **xtask tests** — 84 green
- **cargo xtask check + clippy** — green; the x86_64 warning set is exactly the documented pre-existing family (flags/cli/spl*), zero warnings in any touched file
- Acceptance greps: `fn cpuid`==1, `fn inb`==1, EOI/0xB0==11, LINT==7, `0x6E0`==5 with the 0x832 constant gone, cpuid mentions in timer.rs==19, periodic==18, `unhandled`==1, `cpu::sti`==1 with the ordering comment, SyncUnsafeCell==2

## Known Stubs
None — the polled (LSR-swept) console RX is this phase's designed shape, not a stub; IRQ-driven RX was explicitly out of scope and belongs to a later phase.

## Threat Flags
None — the dispatch-3 fix changes the access path to the same registers (MSRs vs MMIO), adding no new trust-boundary surface beyond the plan's threat model (T-13-05/T-13-06 mitigations verified live: handlers armed before sti on every leg; the CPUID gate host-tested both ways).

## Authentication Gates
None - no authentication was required.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- 13-04 (ACPI/ECAM) starts from `Facts.acpi_root` (rsdt 0x3ffe2345 live on the gate log) with a ticking dispatch underneath; the lapic registry's LVT surfaces are ready for any device vector
- 13-05 (IOAPIC/INTx) inherits the EOI-after-work ordering pinned here (Pitfall 5) and the spare 0x20..=0x3F vector span; the IOAPIC stays MMIO (classic) regardless of the LAPIC's x2APIC mode
- X64-02 remains open by design: this plan delivered its LAPIC/timer substrate; the IOAPIC/under-load halves are 13-05's (the shared-ID gate holds the row until that SUMMARY exists)
- No blockers.

## Self-Check: PASSED

- All 9 r9 key-files exist on disk (`[ -f ]` each: FOUND); r9 working tree clean
- r9 commits verified: c39a210, 0877f3a, 333b940, 607a7c5, 351287f are ancestors of r9 `main` HEAD (`git merge-base --is-ancestor`)
- TDD gates: test(13-03) 0877f3a precedes feat(13-03) 333b940 in ancestry; RED evidence at `13-03-red-evidence.json`
- Production code lives in the sanctioned sibling repo /home/req/code/r9 per the dual-repo execution context; the harvey-side commit for this plan is the docs commit carrying this SUMMARY/STATE/ROADMAP/REQUIREMENTS/deferred-items

---
*Phase: 13-r9-x86-64-platform-bring-up*
*Completed: 2026-11-07*
