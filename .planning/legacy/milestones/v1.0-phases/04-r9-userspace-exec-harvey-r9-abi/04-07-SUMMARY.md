---
phase: 04-r9-userspace-exec-harvey-r9-abi
plan: 07
subsystem: kernel/proc
tags: [r9, aarch64, scheduler, preemption, rfork, await, exits, cross-repo-abi, d13]

requires:
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 04)
    provides: Proc scaffolding (kstack/root_table/frame/context/fd_table/mapping_log), PROC_TABLE, current_proc
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 05)
    provides: no-switch trap entry, factored trap_return tail, exec_bootstrap
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 06)
    provides: syscall dispatch table + handlers (sargs/copy_user_str/validate_user_range), devcons/devroot devices, exec profile harness
provides:
  - Per-process trap-stack discipline: trap.S entry records the live frame into CURFRAME; the trap_return tail reloads the frame base from CURFRAME and, on EL0t-bound returns, loads SP_EL1 from CURKSTACK (the current proc's kstack top) — a blocked-mid-syscall proc's frame can never be overwritten by another process's traps (T-04-14)
  - Tick preemption: timer tick (1 kHz, 1 ms slice) drives proc::preempt → reschedule — round-robin over PROC_TABLE with the CURKSTACK+CURFRAME+TTBR0+current swap as one PROC_TABLE-locked step; one `sched: switch old->new` marker per switch (D-13 marker-not-order)
  - Blocking syscalls: sched()/sched_loop()/pick_next() — swtch-based kernel-context switches on the legacy interrupt stack; the sched loop applies the same locked swap before resuming; EL0 entry/exit never rides swtch
  - swtch.S rewritten symmetric (the prior eret-based sequence was never called on aarch64 and could not round-trip)
  - RFORK(19): donor sysrfork flag validation + eager page-for-page address-space copy (no COW, T-04-15) + dup'd fd table + forkchild frame (x0=0)
  - AWAIT(47)/EXITS wait records: block-via-swtch, Plan-9-shaped `{pid} 0 0 0 "{msg}"` record, EXITS posts + wakes + reschedules; ERRSTR(41) for libc's %r
  - Arg width discipline: int/uint-width syscall args masked to 32 bits (donor va_arg semantics; 7c leaves stack garbage in the upper slot half)
  - config_virtmulti.toml + xtask multi boot-test profile (presence-based smoke markers + ≥1 sched-switch + negative scan)
  - build/aarch64/smokefork.c (fd-ops + rfork + await smoke) and the 04-07 init job (rfork A/B + calibrated spin + await both)
affects:
  - 04-08 (notes at the trap_return boundary ride the same CURFRAME tail; NOTIFY/NOTED join the dispatch table; the blocking machinery here is what note delivery blocks on)
  - Phase 5 (real init stands on swtch-based blocking syscalls + wait records; the masked-boot invariant holds: PSTATE.I=0 ⟺ EL0 or the stateless sched wfi)
  - deferred-items.md #4 (heisenbug root-cause note — revisit before adding any unmasked-EL1h live-computation window)

actuals:
  tokens: 19424
  tasks: 2
  commits: 3
  commits_r9: 2
  commits_harvey: 1
  plan_head_before_r9: 230fc7486a3d88fdbd2058068319e8645f731899
  plan_head_after_r9: fdac9fe
  plan_head_before_harvey: ee52650b4
  time_slice: "1 ms (TICKS_PER_SEC=1000 — see Decisions)"
  sched_switch_evidence: "3–6 `sched: switch` lines per virtmulti run (7 runs sampled)"

tech-stack:
  added: []
  patterns:
    - "trap frame record/reload pair: entry writes CURFRAME, the return tail reloads it — preemption swaps CURFRAME/CURKSTACK/TTBR0 mid-trap and the tail pops the NEW current proc's frame; EL0t-bound returns load SP_EL1 from CURKSTACK explicitly (the entry-side switch is impossible on aarch64: no GPR may be clobbered before its store and there is no GPR-free SP move)"
    - "two resume paths, never mixed: Ready-without-kernel-context procs ride trap_return (the only EL0 entry); blocked-mid-syscall procs ride swtch from the sched loop; the sched loop is stateless between scans, so re-entry from the seed is as correct as resumption after a swtch-out"
    - "the locked swap ordering (checker revision 1): current-proc pointer + CURKSTACK + CURFRAME + TTBR0 change as one PROC_TABLE-locked step BEFORE any resume"
    - "syscall arg width discipline: mask int/uint-width args to 32 bits (the donor's va_arg(uint) truncates per-prototype; 7c's 32-bit store leaves garbage in the slot's upper half)"
    - "exec_bootstrap is a non-preemptible boot critical section: IRQ unmask deferred to just before the first EL0 entry (nothing in the orchestration needs interrupts)"

key-files:
  created:
    - ../r9/aarch64/lib/config_virtmulti.toml
    - build/aarch64/smokefork.c
  modified:
    - ../r9/aarch64/src/trap.S (CURFRAME record at entry; CURFRAME reload + per-origin final sp at the tail; trap_return_exec publishes CURFRAME)
    - ../r9/aarch64/src/swtch.S (symmetric rewrite) + swtch.rs (signature)
    - ../r9/aarch64/src/proc.rs (CURKSTACK/CURFRAME statics, init_trap_stacks, activate_first, preempt, reschedule, sched, sched_loop, pick_next, sched_from_moribund, post_wait_record, rfork_clone, map_kernel_alias; ProcState::Blocked; parent/children_live/nowait/blocked_in_kernel/wait_records fields)
    - ../r9/aarch64/src/platform/virt/timer.rs (tick → preempt; 1 kHz slice; 1 Hz print divider)
    - ../r9/aarch64/src/main.rs (init_trap_stacks at boot; activate_first; dev_exec unmask moved into exec_bootstrap)
    - ../r9/aarch64/src/l.S (boot stack 16 → 64 KiB)
    - ../r9/aarch64/src/syscall.rs (RFORK/AWAIT/ERRSTR handlers, EXITS wait-record+reschedule, int-width arg masks)
    - ../r9/port/src/dat.rs (Chan::dup)
    - ../r9/xtask/src/main.rs (INITRD_FILES + smokefork, BootStage::MultiMarkers, BootProfile::Multi, name-based selection)
    - ../r9/HowItWorks.md (the 04-07 section — file-header mandate)
    - build/aarch64/init.c (rfork A/B + calibrated spin + await both)
    - build/aarch64/stage-build.sh (STAGE_SRCS += smokefork)
    - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md (#4 added)

key-decisions:
  - "Return-side CURKSTACK load instead of the plan's literal entry-side stack switch: an entry-side switch is impossible on aarch64 without clobbering a user GPR before its store (it would reintroduce the exact 04-05 x0-clobber bug the plan's own read_first documents as removed); the tail loading SP_EL1=CURKSTACK on EL0t-bound returns is the 9front-shape equivalent and the single enforcement point of the entry-stack invariant"
  - "swtch.S rewritten symmetric (ret-based, Context-struct layout): the existing sequence pushed x19..x30 in the same order it popped them — on a descending stack every register would resume from a different slot — and sourced SPSR/ELR from stack garbage; it had never been called on aarch64 (latent since import)"
  - "Time slice 1 ms (1000 Hz): measured runs showed the whole userland era completes in a few ms of wall time under TCG and a slower (10–1000 ms) slice can expire only while IRQ-masked — zero preemptions; the not(dev_exec) print is divided back to 1 Hz so the Phase-3 virt profile's pacing is unchanged"
  - "init carries a calibrated volatile spin between the rforks and the awaits: preemption evidence needs a CPU-bound EL0 window with live peers — the children otherwise finish inside one slice, and awaiting parents leave no Ready switch target (the first run without it produced zero sched:switch lines)"
  - "exec_bootstrap runs with IRQs masked, unmasking at the first EL0 entry — see deviation #3 (the heisenbug hunt); nothing in boot orchestration needs interrupts"
  - "ERRSTR(41) implemented: libc's %r (rerrstr calls errstr twice) is a measured caller observed in the first failing virtmulti run — D-14's measured-caller rule satisfied by evidence, not guessing"
  - "RFORK eager-copy always; the donor's RFMEM share-semantics deliberately not honored (the smoke does not depend on sharing; T-04-15 mandates no shared writable pages); fd table value-dup'd (streaming offsets diverge after fork — documented approximation)"
  - "wait records queue per parent (VecDeque), RFNOWAIT honored (no record posted); AWAIT with no children errors 'no children' (donor pwait)"

patterns-established:
  - "preemption evidence by marker (`sched: switch old->new`), never ordering (D-13) — the multi profile asserts ≥1 occurrence"
  - "boot critical-section masking: unmask_irq at the last moment before first EL0 entry under dev_exec"

requirements-completed: [R9-02]

coverage:
  - id: T1
    description: "trap.S CURKSTACK/CURFRAME machinery (entry record + tail reload + EL0t-bound SP_EL1 from CURKSTACK, both symbols initialized to the legacy interrupt stack); preempt()/reschedule() round-robin with the locked CURKSTACK+CURFRAME+TTBR0+current swap and the `sched: switch` marker; swtch-based blocking path (sched/sched_loop/pick_next) with the sched loop applying the same locked swap before resuming; EL0 never rides swtch; tick→preempt wiring; virtexec/virt/virtpanic + raspi3b/raspi4b builds green"
    requirement: "R9-02"
    verification:
      - kind: integration
        ref: "cargo xtask build virtexec/default/raspi4b + clippy virtexec green; plan grep chain (CURKSTACK+CURFRAME in trap.S; preempt/reschedule/swtch/sched: switch in proc.rs; preempt in tick) PASS; boot-test virtexec PASS (SC#2 chain intact — the stack-target change invisible to single-proc), virt PASS (≥3 ticks + echo at the divided 1 Hz print), virtpanic PASS (≥30 register rows + clean halt); port 12/12 + aarch64 host 8/8 tests"
      status: pass
    human_judgment: false
  - id: T2
    description: "RFORK (donor flag validation, eager KZERO-alias page copy, fd dup, forkchild x0=0) + AWAIT (blocks via swtch, Plan-9-shaped record) + EXITS (posts record, wakes parent, reschedules) + smokefork binary + init rfork/spin/await job + config_virtmulti profile; evidence: the virtmulti log carries every smoke marker, ≥1 sched:switch line, clean all-exited, zero panic/Unhandled lines; smokefork's fd check proves OPEN/PREAD/CLOSE against devroot from EL0; virtexec stays green against the evolved init; Phase-3 profiles + raspi builds green"
    requirement: "R9-02"
    verification:
      - kind: integration
        ref: "stage-build.sh PASS (init+smokefork compiled, R_MAGIC 00008c47 verified per binary); cargo xtask initrd (bootfs.paq sha256 a8e4f6b9…); boot-test virtmulti PASS — 7/7 final-state runs, each carrying: hello aarch64 harvey, smokefork: fd ok, smokefork: child running, smokefork: await: 4 0 0 0 \"kid\", init: reaped 2/3 records, init: smoke complete, all user processes exited, 3–6 `sched: switch` lines, zero panic/Unhandled; plan's explicit grep legs on the log PASS incl. panic-count==0; regression fan at final HEADs: virtexec PASS ×4 (+30/30 raw boots), virt PASS, virtpanic PASS, default+raspi4b builds green, clippy virtexec green, xtask 19/19 + port 12/12 + aarch64 host 8/8"
      status: pass
    human_judgment: false

duration: 240min
completed: 2026-11-02
status: complete
---

# Phase 04 Plan 07: Preemptive Multi-Process (Scheduler + RFORK/AWAIT) Summary

**R9-02's process-model half is execution-proven: r9 now runs a small preemptive multi-process system on qemu virt — per-process kernel stacks enforced by the CURKSTACK/CURFRAME trap-frame record/reload, a 1 ms generic-timer tick driving round-robin preemption with `sched: switch old->new` marker evidence (IRQ-from-EL0 returns to a DIFFERENT process — observed 3–6 times per run), RFORK with an eagerly copied address space (no COW) and the donor forkchild shape, and AWAIT/EXITS wait records on the swtch-based blocking path — all proven by the rfork/fd-ops smoke (smokefork) running under preemption with zero panic lines, while virtexec/virt/virtpanic and both raspi builds stay green.**

## Performance

- **Duration:** 240min (the heisenbug hunt — deviation #3 — dominated; ~75min of it)
- **Started:** 2026-11-02T14:38:56Z
- **Completed:** 2026-11-02
- **Tasks:** 2/2
- **Commits:** 2 r9 (`a472f12`, `fdac9fe`) + 1 Harvey (`ab13d0d2b`) + the metadata commit carrying this file
- **Verification runs:** 4 build configs + clippy (multiple states), boot-test virtexec ×7, virtmulti ×7, virt ×3, virtpanic ×3, ~100 raw qemu boots across bisection probes, port 12/12 + aarch64 host 8/8 + xtask 19/19

## Accomplishments

- **Task 1 (r9 `a472f12`) — per-process trap stacks + tick round-robin.** trap.S's entry records the live frame into `CURFRAME` (3 instructions, after the x9/x10 slots are stored); the `trap_return` tail reloads the frame base from CURFRAME and, on EL0t-bound returns, loads SP_EL1 from `CURKSTACK` — the current process's kernel-stack top — the single enforcement point of the per-process entry-stack invariant (T-04-14/Pitfall 10). proc.rs gained `preempt`/`reschedule` (round-robin with the CURKSTACK+CURFRAME+TTBR0+current swap as one PROC_TABLE-locked step, `sched: switch` marker per switch) and the `sched`/`sched_loop`/`pick_next` blocking path (swtch-based, on the legacy interrupt stack; the sched loop applies the same locked swap before resuming — the checker revision-1 ordering assignment). **The existing aarch64 swtch.S turned out to be latent dead code that could never round-trip** (push order mirrored pop order on a descending stack; SPSR/ELR sourced from stack garbage; never called on aarch64 since import) — rewritten symmetric (struct save/restore + `ret`).
- **Task 2 (r9 `fdac9fe`; Harvey `ab13d0d2b`) — RFORK + AWAIT + the preemptive smoke.** RFORK validates flags per donor sysrfork, clones with an eager page-for-page copy through KZERO aliases (T-04-15: no shared writable pages), dups the fd table (`Chan::dup`), mirrors segments + the paqfs byte source, and seeds the child's frame on ITS kstack with x0=0 (parent gets the pid). AWAIT blocks via `proc::sched()` and formats `{pid} 0 0 0 "{msg}"`; EXITS posts the record, wakes the Blocked parent, and hands off to the sched loop (the swtch save writes only the PROC_TABLE-resident context — never the freed kstack). smokefork exercises OPEN/PREAD/CLOSE on /hello's R_MAGIC header from EL0, then rforks+awaits; init rforks /hello + /smokefork, spins (preemption window), and reaps both. The xtask multi profile (config-name-selected, kernel cfg byte-identical to virtexec) asserts presence of all five smoke markers + ≥1 `sched: switch` + all-exited + the always-on negative scan. **Final evidence: 7/7 virtmulti runs PASS with 3–6 preemption switches each; `smokefork: await: 4 0 0 0 "kid"` and `init: reaped N 0 0 0 "…"` records delivered correctly under interleaving.**
- **The gate caught three real bugs** (see Deviations): the never-called swtch.S's broken asymmetry; the int-width syscall-arg upper-slot garbage (await's buflen arrived as `0x7fff0000007f`); and the exec_bootstrap-window tick heisenbug (closed by masked boot).

## Task Commits

| Task | Repo | Commit | Files |
|------|------|--------|-------|
| 1 | r9 | `a472f12` | trap.S, swtch.S, swtch.rs, proc.rs, timer.rs, main.rs |
| 2 | r9 | `fdac9fe` | syscall.rs, proc.rs, main.rs, timer.rs, l.S, port/dat.rs, xtask/main.rs, config_virtmulti.toml (new), HowItWorks.md |
| 2 | harvey | `ab13d0d2b` | build/aarch64/init.c, smokefork.c (new), stage-build.sh |
| metadata | harvey | (final commit) | .planning SUMMARY/STATE/ROADMAP/REQUIREMENTS/deferred-items |

**r9 HEAD at evidence time (D-13 ordering — r9 commits landed first):** `fdac9fe` (on top of Task-1 `a472f12`; the gate ran on the exact committed tree).
**Evidence-run bootfs.paq sha256:** `a8e4f6b9cb45e6a49249f045097ad9a9d9811b4af8d3b2acb852a54c0414a4ab` (per-bake digest — mkpaqfs embeds a time field).

## Files Created/Modified

**r9:** `aarch64/src/trap.S`, `swtch.S`, `swtch.rs`, `proc.rs`, `syscall.rs`, `main.rs`, `l.S`, `platform/virt/timer.rs`, `port/src/dat.rs`, `xtask/src/main.rs`, `aarch64/lib/config_virtmulti.toml` (new), `HowItWorks.md`.

**Harvey:** `build/aarch64/smokefork.c` (new), `build/aarch64/init.c`, `build/aarch64/stage-build.sh`, `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md` (#4 added).

## Decisions Made

See key-decisions in the frontmatter: the return-side CURKSTACK load (the plan's literal entry-side switch is impossible on aarch64 without reintroducing the 04-05 x0-clobber), the symmetric swtch rewrite, the 1 ms slice with the 1 Hz print divider, init's calibrated spin (a preemption window requires a CPU-bound EL0 process with live peers), masked exec_bootstrap, ERRSTR as a measured caller, eager-copy-always rfork semantics, and the per-parent wait-record queue.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug in never-called latent code] swtch.S could not round-trip**
- **Found during:** Task 1 design review (the plan said "reuse as-is")
- **Issue:** the imported aarch64 swtch.S pushed x19..x30 in the SAME order it popped them — on a descending stack every register would resume from a different slot — stored a mid-sequence sp into `*from`, discarded the saved sp on resume, and ended in `eret` with SPSR/ELR sourced from garbage. It had never been called on aarch64 (zero callers since import), so the damage was latent.
- **Fix:** symmetric rewrite against the repr(C) Context layout (save into `from`, load from `to`, `ret` to to.x30); Rust signature became `swtch(from: &mut Context, to: &Context)`.
- **Files modified:** ../r9/aarch64/src/swtch.S, swtch.rs
- **Verification:** the AWAIT block/resume cycle round-trips correctly in the smoke (parent blocks, children run, parent resumes mid-syscall with its kstack frame intact); all boot profiles green.
- **Commit:** a472f12 (r9)

**2. [Rule 1 — Bug caught by the gate] int-width syscall args arrive with garbage upper slot halves**
- **Found during:** Task 2 first virtmulti run — both awaits returned -1; ERRSTR instrumentation revealed `await: bad user buffer`, and a debug print showed buflen = `0x7fff0000007f` (low32 = 127, correct).
- **Issue:** 7c materializes int/uint-width arguments with a 32-bit store into the 8-byte arg slot, leaving the upper half as stack garbage; the donor's `va_arg(list, uint)` truncates per-prototype, but r9's handlers read full u64 slots. init's first await survived only because its seed stack page was still zero-filled.
- **Fix:** mask int-width args to 32 bits per the frozen prototypes (await's buflen, errstr's n, rfork's flag, the fd args; open's mode was already masked) + the width-discipline rule documented on `sargs`. ERRSTR(41) was added — libc's `%r` (rerrstr calls errstr twice) is its measured caller, observed in the failing log.
- **Files modified:** ../r9/aarch64/src/syscall.rs
- **Verification:** the full smoke then completed end-to-end (all markers incl. both reaps); the failure print paths now carry real errstrs.
- **Commit:** fdac9fe (r9)

**3. [Rule 1/3 — Blocking heisenbug, closed by construction after a full bisection] intermittent inflate corruption when a tick landed inside exec_bootstrap**
- **Found during:** Task 2 regression fan — virtexec intermittently failed with a bare `panic` before the exec-entry marker (~5–30% rate, layout-sensitive).
- **Investigation (each step evidence-backed):** captured full panic: `exec: /init: paqfs: inflate failed` (and twice a shift-overflow panic INSIDE miniz at core.rs:731/:1701); retry-on-byte-identical-input always succeeded (computation corrupted mid-flight, image intact); a stack+register+SIMD canary under the same 1 kHz tick stayed clean 20/20; the tick handler path provably writes only BSS statics + the trap frame + MMIO and uses no FP/SIMD; stack low-water showed ~55 KiB of headroom (the 16→64 KiB l.S bump I made mid-hunt was kept as headroom insurance but was not the fix); allocator/mcslock exclusive sequences retry correctly. **Control experiment:** the 04-06-final tree + 1 kHz tick = 0/30 clean; my tree = hits — the corruptor was in the 04-07 delta, but reverting trap.S piece by piece produced only noise-level rate changes (heisenbug shifted with code layout). Root cause NOT isolated (candidates: qemu TCG IRQ delivery at high rate, an unspotted trap.S state interaction) — recorded as deferred-items.md #4.
- **Fix (by construction):** exec_bootstrap now runs with IRQs MASKED and unmasks only just before the first EL0 entry — the correct design regardless (nothing in boot orchestration needs interrupts; Plan 9 boots with interrupts off until late). The system's invariant becomes: `PSTATE.I=0 ⟺ at EL0, or in the stateless sched-loop wfi window` — every inflate/syscall/fault path runs IRQ-masked, so the corruption window is structurally eliminated while D-13's tick-from-EL0 deliverable is unaffected.
- **Files modified:** ../r9/aarch64/src/main.rs (unmask moved), l.S (stack bump retained)
- **Verification:** 30/30 raw virtexec boots + 4/4 virtexec boot-tests + 7/7 virtmulti boot-tests clean at the final state; regression fan green.
- **Commit:** fdac9fe (r9)

**4. [Rule 2 — Missing measured syscall] ERRSTR(41)**
- **Found during:** deviation #2's diagnosis (libc's `%r` printed empty errstrs: `syscall 41 unimplemented` ×2 per failure print).
- **Fix:** handler added (copy proc.errstr NUL-terminated into the validated user buffer, return 0) — a measured caller per D-14, documented in the table comment.
- **Commit:** fdac9fe (r9)

**Plan-supporting adjustments (in-plan intent):** the plan's Task-1 action sketched an entry-side `ldr x0, =CURKSTACK; mov sp, x0` — that literal sequence would reintroduce the exact 04-05 x0-clobber fatality (any GPR clobbered before its frame store destroys that register's slot); the intent (per-process entry stack) is delivered via SP_EL1 maintenance with the return-side CURKSTACK load as the enforcement point — trap.S comment + HowItWorks.md document this. The preemption evidence requirement motivated init's calibrated spin (the smoke otherwise finishes inside one slice with no Ready peer to switch to). The wait-record slot grew into a per-parent queue (two children exit before the parent runs). TICKS_PER_SEC moved to 1000 with a print divider (the Phase-3 virt profile keeps its 1 Hz pacing). l.S's boot stack grew to 64 KiB (headroom insurance for the deep exec_bootstrap chains; retained after the masked-boot fix).

**Total deviations:** 4 auto-fixed + the plan-supporting adjustments. No Rule 4 (architectural) items — the masked-boot change is a boot-sequencing fix within the existing design, not a structural change.
**Impact on plan:** none to intent — every must-have truth, acceptance criterion, and plan-level verification item passes at the final state.

## Known Stubs

None introduced. The remaining 04-04-era seams (note_queue, notify_handler) belong to 04-08 per its plan. The `spsr` field of swtch Context is saved/restored as a slot but unused by the ret-based switch (documented in swtch.S).

## Issues Encountered

- The heisenbug (deviation #3) consumed most of the plan's duration: the failure's ~5–30% rate swung with code layout, so bisect-by-removal gave noisy signal until the control experiment (04-06 tree + 1 kHz) cleanly isolated the 04-07 delta; the definitive closure is the masked-boot invariant. deferred-items.md #4 carries the root-cause note.
- macOS `sed -i` vs GNU semantics cost one failed probe (no tree impact).
- Pre-existing rustfmt drift across untouched files (exec.rs, trap.rs, devroot.rs, uregcheck.rs, config.rs, …) — only my own hunks were formatted; the rest is left alone per the scope boundary.

## Threat Flags

None new beyond the plan's register: T-04-14 landed as designed (per-proc kstack discipline via CURKSTACK/CURFRAME; the locked swap; the regression matrix green). T-04-15 landed (eager page-for-page copy into fresh phys pages mapped only in the child's root table; fd table value-dup'd; the smoke's child-exit/parent-await shape proves isolation). T-04-SC: no package installs this plan (no new crates).

## User Setup Required

None.

## Next Phase Readiness

- **04-08 (notes):** the blocking machinery it stands on is proven — a proc blocked mid-syscall keeps its frame on its own kstack and resumes via the sched loop's swtch; NOTIFY(28)/NOTED(29) join the dispatch table (the EXEC tos_va convention shows the shape for NOTED's special case); note delivery rewrites the frame at the trap_return boundary, which is now CURFRAME-driven (delivery must update the proc's stashed frame — same object).
- **Round-robin + time-slice knob:** TICKS_PER_SEC=1000 in `platform/virt/timer.rs`; the multi profile's assertion idiom generalizes to note smoke markers.
- **The masked-boot invariant** (`PSTATE.I=0` only at EL0 or the stateless sched wfi) must be preserved by any new boot/orchestration code — deferred-items.md #4 says why.
- r9 HEAD for downstream pinning (D-13): `fdac9fe`.
- Flagged assumption carried from the plan: R9-02 remains probe-unclassified — acceptance derived from ROADMAP SC#4 + CONTEXT D-13 and the 04-01 trace artifact; the verifier should treat the probe row as a flagged assumption, not a resolved edge.

## Self-Check: PASSED

- Key files exist: r9 `aarch64/lib/config_virtmulti.toml` ✓ (committed); Harvey `build/aarch64/smokefork.c` ✓ `init.c` ✓ `stage-build.sh` ✓; this SUMMARY ✓.
- Commits: r9 `git log --grep="04-07"` → `a472f12` (Task 1) + `fdac9fe` (Task 2) ✓; Harvey → `ab13d0d2b` ✓ (plus the metadata commit carrying this file).
- Acceptance criteria re-run at final HEADs: Task 1 (4-config builds + clippy green; grep chain PASS; boot-test virtexec/virt/virtpanic PASS) ✓; Task 2 (stage-build PASS with per-binary R_MAGIC check; initrd bake; virtmulti PASS ×7 final-state runs with 3–6 `sched: switch` each and zero panic/Unhandled lines; explicit log grep legs PASS; regression fan virtexec/virt/virtpanic/raspi3b/raspi4b green) ✓.
- Plan-level verification at final HEADs: D-13 delivered (tick-preemptive round-robin, IRQ-from-EL0 exercised, markers-not-order asserted); the blocking-syscall machinery (per-proc kstacks + swtch) proven by AWAIT; port 12/12 + aarch64 host 8/8 + xtask 19/19.

---
*Phase: 04-r9-userspace-exec-harvey-r9-abi*
*Completed: 2026-11-02*
