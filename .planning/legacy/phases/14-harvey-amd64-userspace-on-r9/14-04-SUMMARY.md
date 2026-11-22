---
phase: 14-harvey-amd64-userspace-on-r9
plan: 04
subsystem: exec-substrate-runtime
tags: [proc, scheduler, syscall, dispatch, x86_64, r9, pml4, cr3, tss-rsp0, xexec-02]
duration: 48 min
completed: 2026-11-08
requires:
  - "14-03-complete r9 HEAD 822b3ec (usermem's UserAddressSpace predicate, the amd64 a.out loader, the 184-byte Ureg seed)"
  - "14-02's armed entry trio (syscallentry/syscallreturn/touser) and its fillable dispatch stub"
provides:
  - "x86_64/src/proc.rs — Proc/segments/scheduler/rfork/await/notes on the per-process PML4/CR3 + TSS-rsp0 model (the OQ-2 resolution, kernel higher half shared from the boot PML4), validate_user_range shape-verbatim, Proc implements usermem's UserAddressSpace"
  - "x86_64/src/syscall.rs — the measured-set SYSCALL dispatch (BP number, stack-range-validated SP+8 args, AX result, NSEC no-stack-read, _seek/_exits under TEXT names, loud-unimplemented default) + deliver_notes (arg0 through ureg->bp) + exit_tail"
  - "x86_64/src/exec.rs mapping bodies — install_image/map_text_eager/map_data_bss_eager/seed_user_pages (order-stable argv), exec_into/exec_replace, read_aout's backing-length check"
  - "the 14-02 entry stub wired to the real dispatch; the tick println cfg-gated off under dev_exec; trap_return/idle_wait/park_forever + swtch.rs (the scheduler's substrate)"
affects:
  - "../r9/x86_64/src/{proc,syscall,exec,trap,swtch,dat,seg,pagealloc,main}.rs, ../r9/x86_64/src/platform/q35/timer.rs, ../r9/x86_64/src/l.S, ../r9/x86_64/Cargo.toml"
  - "14-05 (exec_bootstrap consumes exec_into/seed frames/touser + the dev_exec tick arms), every Phase-14 ring-3 plan"
tech-stack:
  added: []
  patterns:
    - "per-process PML4 with a SHARED kernel half (boot-table entries 510/511 copied at construction) + a fixed kernel-half kstack window (PML4[400], kernel-only, US clear) reached through TSS rsp0 — no physically-contiguous allocation, no dynamic kernel-table aliasing (KZERO+PA covers all RAM)"
    - "the physpage seam: kernel side = pagealloc + the KZERO alias (1 GiB boot-map window, loud guard); host-test side = real host pages as PAs with an identity alias — the mapping/exec/rfork bodies are genuinely host-exercised, not deferred to metal"
key-files:
  created:
    - ../r9/x86_64/src/proc.rs
    - ../r9/x86_64/src/syscall.rs
    - ../r9/x86_64/src/swtch.rs
  modified:
    - ../r9/x86_64/src/exec.rs
    - ../r9/x86_64/src/trap.rs
    - ../r9/x86_64/src/dat.rs
    - ../r9/x86_64/src/seg.rs
    - ../r9/x86_64/src/pagealloc.rs
    - ../r9/x86_64/src/main.rs
    - ../r9/x86_64/src/platform/q35/timer.rs
    - ../r9/x86_64/src/l.S
    - ../r9/x86_64/Cargo.toml
key-decisions:
  - "per-process PML4/CR3 with the kernel half shared by copying the boot PML4's entries 510 (the KCPUZERO Mach window the gs-relative entry asm needs) and 511 (KZERO = all RAM); kstacks map at a FIXED kernel-half VA (PML4 slot 400, US clear) in each proc's own table so four independently-allocated pages form one VA range — the TSS rsp0 is the one kstack mechanism, re-pointed as a locked step with current-proc + CURFRAME + CR3 (CR3 last: frame pointers are only dereferenced under the new table)"
  - "TEXT maps EAGERLY at exec — the x86_64 deviation from the aarch64 demand-page policy, because no trap-context pager exists on x86_64 this era (the first instruction fetch must find its page); deeper stack/tos growth beyond the two seed pages likewise waits on the pager (recorded in WINDOWS)"
  - "NSEC(53) returns the loud no-RTC errstr on x86_64 (the LAPIC tick count is monotonic-only; no epoch clock exists) — the nsec_now seam is ready for one; the zero-arg no-stack-read discipline is what the behavior test pins"
  - "rfork flag consts homed in proc.rs (aarch64 homes them in syscall.rs) so the Task-1 commit compiles standalone; deliver_notes homed in syscall.rs (aarch64 home: trap.rs) — the note boundary is entirely syscall-module-owned on x86_64, the IRQ-side call reaching it as crate::syscall::deliver_notes"
  - "usize::next_multiple_of replaced with power-of-two masks across proc.rs/exec.rs — it lowers through u128 arithmetic (__multi3), a soft-float link error the moment the syscall chain made the mapping bodies live code"
requirements-completed: []   # XEXEC-02 is declared by 14-02/03/04/05 (shared-ID gate): completes with 14-05
commits: 4
plan_head_before: 822b3eccc1a5949da43a8d626a028773f075d3f5   # r9 repo (all code r9-side; harvey ledger e1198f2098 for the docs commit)
plan_head_after: abee15c367821bebd84415805ccb1f5a2508aef6   # r9 repo
actuals:
  tokens: 57645   # chars/4 over the realized r9 diff (822b3ec..abee15c: 5299+/52-)
  tasks: 3
  commits: 4      # r9 repo (1b2f023, 6977748, 7132059, abee15c); the harvey-side docs commit follows this file
status: complete
coverage:
  - deliverable: "proc.rs — the process machinery on the per-process CR3/TSS-rsp0 model with host tests (Task 1, tracer)"
    verification:
      - kind: command
        ref: "cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu — 58 passed incl. 8 new proc tests (validate_user_range membership, classify shared-page rule, wait-record/argv bookkeeping, RFMEM lifecycle, sem registry, new_user_proc construction, user-page map/unmap round-trip through the seam)"
        status: pass
      - kind: command
        ref: "cargo xtask check + clippy exit 0 (x86_64 warnings A/B-equal at the pre-existing 9, none in new code); q35 boot-test PASS with the machinery compile-present"
        status: pass
      - kind: command
        ref: "tracer gate end-to-end re-run (tests + check + clippy + q35) green — ⚡ Tracer verified end-to-end"
        status: pass
      - kind: command
        ref: "acceptance greps: validate_user_range=18 (>=1), Ureg=27 (>=2), pml4/cr3=97 (>=1), mod proc main.rs=2 (baseline 0), aarch64+port diff empty"
        status: pass
    human_judgment: false
  - deliverable: "syscall.rs — the measured-set dispatch with the amd64 convention and quirks, host-tested RED→GREEN (Task 2, TDD)"
    verification:
      - kind: command
        ref: "RED r9@6977748: 0 passed / 6 failed, every failure on its planned assertion (zeroed buffer vs boom, ax 0 vs -1, ax 0 vs clock, stub errs) — zero accidental greens"
        status: pass
      - kind: command
        ref: "GREEN r9@7132059: 6/6 behavior tests — ERRSTR routed from frame.bp with args from SP+8 upward (SP+0 poison proves the slot source), the arg window validated before copy, loud-unimplemented (named errstr + -1), NSEC zero-arg no-stack-read, staging order, exec_replace's order-stable argv seed read off the mapped Stack page"
        status: pass
      - kind: command
        ref: "cargo xtask check + clippy exit 0 (A/B-equal 9); q35 boot-test PASS; port 101"
        status: pass
      - kind: command
        ref: "acceptance greps: frame.bp=17 (>=1), unimplemented=9 (>=2), mod syscall=1 (baseline 0); the syscall filter reports 6 passed"
        status: pass
    human_judgment: false
  - deliverable: "entry wiring + the tick-print cfg gate + full regression (Task 3)"
    verification:
      - kind: command
        ref: "q35 boot-test PASS twice consecutively — captures 1559 bytes before AND after the wiring (feature-off behavior byte-identical); q35net PASS"
        status: pass
      - kind: command
        ref: "cargo xtask ureg-check — both arches Match + the negative control fired; aarch64 virt PASS (the sibling untouched: aarch64+port diff empty at every commit)"
        status: pass
      - kind: command
        ref: "cargo xtask check + clippy exit 0 (A/B-equal 9); 58 x86_64 host tests + 101 port tests green"
        status: pass
    human_judgment: false
---

# Phase 14 Plan 4: Process Machinery + SYSCALL Dispatch (proc.rs/syscall.rs on the per-process CR3/TSS-rsp0 model) Summary

Ported the process machinery (Proc/scheduler/rfork/await/notes on per-process PML4s with the kernel half shared and kstacks behind TSS rsp0) and the measured-set SYSCALL dispatch (BP number, stack-range-validated SP+8 args, AX result, NSEC no-stack-read, loud-unimplemented default) to the r9 x86_64 crate, wired the 14-02 entry asm to the real dispatch, and cfg-gated the tick println off under dev_exec — host-verified end-to-end with the full Phase-13 regression battery green and feature-off boots byte-identical.

**Duration:** 48 min (2026-11-08T20:04:52Z → 2026-11-08T20:52:28Z)
**Tasks:** 3/3 complete
**Files:** 12 r9-side (3 created, 9 modified); nothing Harvey-side beyond this summary

## Commits

| Repo | Hash | Message |
|------|------|---------|
| r9 | 1b2f023 | feat(14-04): port the process machinery to the per-process CR3 + TSS-rsp0 model |
| r9 | 6977748 | test(14-04): the syscall behavior battery — RED against the dispatch skeleton |
| r9 | 7132059 | feat(14-04): the measured-set SYSCALL dispatch — BP number, SP+8 args, AX result |
| r9 | abee15c | feat(14-04): wire the entry asm to the real dispatch + the tick-print cfg gate |

## Accomplishments

### Task 1: proc.rs port — Proc/scheduler/rfork on the per-process CR3 + TSS-rsp0 model (tracer)

- **The address-space re-key (OQ-2's resolution implemented):** every Proc owns a PML4 whose kernel half is SHARED — construction copies the boot PML4's entries 510 (the KCPUZERO Mach window; without it the gs-relative SYSCALL entry could not reach the Mach under a per-process table) and 511 (KZERO = all RAM) from the live boot table (l.S's bootpml4, now .globl). The aarch64 `root_table: *mut RootPageTable` becomes `pml4_pa: u64` (the CR3 value); `map_to` becomes a local four-level walk that allocates intermediates lazily and RETURNS them so the owning proc's exit free covers them (a `mapping_tables` Vec — the aarch64 single-page root table amortized its intermediates; x86_64 cannot).
- **The kstack re-key:** aarch64 needed PA-contiguous bump-allocator pages; x86_64 maps four independently-allocated pages at a FIXED kernel-half VA (PML4[400], 0xffff_3200_0000_0000) in each proc's own table, kernel-only (no PTE_US — ring 3 can never reach it even though the VA is known). The TSS rsp0 is re-pointed at it on every switch — the ONE kernel-stack mechanism both ring-3 entry paths share, replacing aarch64's CURKSTACK side channel entirely. Kernel-side content writes ride the KZERO+PA alias (the 1 GiB boot-map window, loud guard) — the "kernel higher half shared" model makes the aarch64 dynamic kernel-alias mapping a no-op.
- **Everything else straight-ported:** validate_user_range shape-verbatim (overflow/zero-length/one-segment-membership guards + the single-process-era caveat comment, re-keyed to the CR3/live-memory justification), the round-robin scheduler (preempt/reschedule/pick_next with the full switch as one locked step — state, current, rsp0, CURFRAME, CR3-last), sched/sched_loop/sched_from_moribund over a new swtch.rs (dat.rs's Label as the context; callee-saved + sp + a shared resume label, offset-witnessed), rfork_clone/rfork_copy (the RFMEM share arms and bss sweep re-keyed to the PML4 walk; the child frame copies through the child's top kstack page's kernel alias — the frame VA lives in the CHILD's table — with `ax = 0` as the child wire result), await/wait records, the note machinery, sem/RFMEM registries, all the device hooks (pipe/cons/rendez/devproc/devdup/devsrv/env). aarch64's per-page CTR_EL0 icache step is deliberately absent (coherent icache on x86) — the re-key the plan names. `Proc` implements 14-03's `UserAddressSpace` trait, so the usermem choke point plugs in with zero interface churn.
- **Support files the port needed (documented deviation):** trap.rs grew `trap_return` (intrcommon's restore tail as a free function — the ONLY way the scheduler enters CPL3, with a conditional SWAPGS on the saved CS RPL), `idle_wait` (sti/hlt/cli), `park_forever`; seg.rs grew `Tss::set_rsp0_raw`; dat.rs's Label fields went pub(crate) and Ureg's wire slots (ax/bp/sp/pc/cs/flags/ss) pub(crate); pagealloc grew `deallocate_physpage`; Cargo.toml predeclares the dev_exec/dev_bootchain cfgs.
- **8 host tests** through the physpage seam (kernel: pagealloc+KZERO alias; host: real pages as PAs, identity alias — the 14-03 mock doctrine extended to the mapping bodies): validate_user_range membership edges, the classify data/bss shared-page rule, wait-record/argv bookkeeping, RFMEM lifecycle, sem consume/reregister, new_user_proc construction (the PML4/kstack window/preopen), a user-page map/unmap round-trip walking all four levels. My first test draft's read-back walk skipped the PT level and the classify mock's bss was swallowed by the shared-page rule — both test bugs fixed before commit (the implementation was right; the debug walk proved it).

### Task 2: syscall.rs — the measured-set dispatch with the amd64 convention and quirks (TDD)

- **RED (r9@6977748):** the module skeleton (frozen sys.h consts, the dispatch entry shapes, the nsec clock seam, the exec staging seam) + exec.rs's exec_replace stub + 6 behavior tests — 0 passed / 6 failed, every failure on its planned assertion, zero accidental greens.
- **GREEN (r9@7132059):** the aarch64 dispatch port with the amd64 re-keys — number from `frame.bp`, result into `frame.ax`, NOTED's no-writeback special case, and the loud-unimplemented default (`syscall {n} unimplemented` + errstr + -1). **sargs implements the donor's order (T-14-06):** the window `[sp, sp+8+8*arity)` — the return-address slot INCLUDED — validates through usermem's `validated_only` BEFORE any slot is copied upward from SP+8; the SP+0-poison test proves a generic SP+0 reader would have failed. The 32-bit-arg masking discipline comment carried. The quirks: NSEC(53) dispatched with NO stack read (the clock seam returns the loud no-RTC error — no epoch clock on x86_64; the test-hook clock pins the zero-arg register-return), SEEK/EXITS under their _-prefixed names with unchanged numbers.
- **The full 30-handler measured set** over the port devices (pread/pwrite, open/bind/chdir/unmount/stat/fstat/mount/fversion, exec/rfork/await/exits/notify/noted, dup/pipe/create/fd2path, brk_/semacquire/semrelease, alarm/sleep, seek/errstr/rendezvous, nsec) — near-verbatim ports; sys_rfork against Task 1's machinery, sys_exec staging argv through the factored order-stable `copy_exec_args` then exec_replace.
- **exec.rs's mapping bodies (the half 14-03's module doc assigned here):** install_image (segments/byte-source/argv records), map_text_eager (the documented deviation), map_data_bss_eager (zeroed pages + clamped file content through the kernel alias), seed_user_pages (the aarch64 top-down argv layout — order-stable, pinned byte-for-byte by the test reading the mapped Stack page back), exec_into/exec_replace (parse→free→seed→rewrite-the-live-frame), read_aout's backing-length check.
- **deliver_notes + exit_tail:** the donor 9k k10 note redirect re-keyed — arg0 rides `ureg->bp` (RARG, ABI.md's convention), the pushed copy is the whole 184-byte frame, the write window validated + forced resident before any write, no-handler kills via the exit tail. exit_tail (fd-group sweep, frees, wait record, all-exited park) is the shared tail for exits/NDFLT/the post-free exec kill. dat.rs gained `copy_to_saved`/`restore_from_saved`/`force_user_return` — the amd64 privilege force (cs/ss = flat user selectors, IOPL cleared, IF set): SYSRETQ derives CS/SS from STAR, but RFLAGS rides R11 straight from the user-editable saved frame, so a handler-edited IOPL would grant I/O privilege without it.

### Task 3: entry wiring + the tick-print cfg gate + full regression

- **trap.rs:** the 14-02 fillable stub's body became `crate::syscall::entry` (dispatch + the note boundary) — the asm's CALL target and argument order untouched. A ring-3-origin sync exception under dev_exec now names the vector and kills the proc via exit_tail (the donor faultnote shape) instead of halting the kernel — with text/data/bss eager, a user fault is a user bug.
- **timer.rs:** the tick println cfg-gated OFF under dev_exec (Pitfall 8 — syscall context takes the Console MCS lock; the aarch64 exec-profile precedent) and the tick's dev_exec arms wired (tick_timeouts + preempt, no-ops until the first proc exists). Feature-off boots are byte-identical: the q35 gate captured exactly 1559 bytes before and after the wiring.
- **Two live-code link fixes (documented):** `usize::next_multiple_of` lowers through u128 arithmetic (`__multi3` — soft-float in the SSE-free kernel), which became a link error the moment the syscall chain made the mapping bodies reachable; replaced with power-of-two masks across proc.rs/exec.rs. And l.S's `bootpml4` went `.globl` so proc.rs can read the kernel-half entries.
- **Full regression:** q35 ×2 (byte-identical captures), q35net, ureg-check (both arches Match + negative control fired), aarch64 virt, check/clippy (x86_64 warnings A/B-equal at the pre-existing 9), 58 x86_64 host tests, 101 port tests — and `git -C ../r9 diff --stat HEAD -- aarch64/ port/` empty at every commit (the frozen sibling untouched).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The port needed support surfaces beyond the plan's files_modified list**
- **Found during:** Task 1
- **Issue:** the aarch64 proc.rs compiles against crate-local machinery with no x86_64 home — `crate::swtch` (kernel-context switch), `trap_return`/`idle_wait`/`park_forever` (the scheduler's resume/idle/halt arms), a per-process-page free path in pagealloc, and a TSS rsp0 write API.
- **Fix:** created swtch.rs (Label-based, offset-witnessed); added the three trap.rs fns, `Tss::set_rsp0_raw`, `deallocate_physpage`, Label/Ureg visibility, and the dev_exec/dev_bootchain check-cfg predeclarations. Every file is the natural home the plan's own read_first list names (trap.rs/seg.rs as "the mechanism" sources).
- **Files modified:** ../r9/x86_64/src/{swtch.rs(created),trap.rs,seg.rs,dat.rs,pagealloc.rs,Cargo.toml}
- **Commit:** r9@1b2f023

**2. [Rule 2 - Policy] TEXT maps eagerly at exec (deviation from the 14-03-committed demand-text policy)**
- **Found during:** Task 2 (exec.rs mapping bodies)
- **Issue:** the 14-03 module doc carried aarch64's demand-paged-text policy, but no trap-context pager exists on x86_64 this era — the first instruction fetch would #PF into the dump-and-halt, dead-ending 14-05's marker gate.
- **Fix:** map_text_eager maps every text page with its file bytes at install; the module doc now records the deviation and the follow-up (text demand paging + deeper stack/tos growth arrive with a pager). Recorded in WINDOWS for ship-gate visibility.
- **Files modified:** ../r9/x86_64/src/exec.rs
- **Commit:** r9@7132059

**3. [Rule 3 - Blocking] Live-code link errors when the entry wiring made the chain reachable**
- **Found during:** Task 3 verify
- **Issue:** (a) `usize::next_multiple_of` lowers through u128 arithmetic → `__multi3`, undefined in the SSE-free soft-float kernel — the mapping bodies were dead code until the syscall chain linked them; (b) proc.rs's extern reference to l.S's `bootpml4` failed to resolve (the label was local).
- **Fix:** power-of-two mask round-ups across proc.rs/exec.rs (both alignments are powers of two — exact, not approximate); `.globl bootpml4` with the sharing rationale in place.
- **Files modified:** ../r9/x86_64/src/{proc.rs,exec.rs,l.S}
- **Commit:** r9@abee15c

**4. [Rule 3 - Placement] Const/fn homes adjusted for commit ordering and module ownership**
- **Found during:** Tasks 1-2
- **Issue:** the rfork flag consts live in aarch64's syscall.rs, but Task 1's proc.rs references them and Task 1 must compile standalone; deliver_notes lives in aarch64's trap.rs but x86_64's note boundary is entirely syscall-module-owned.
- **Fix:** the flag consts homed in proc.rs (syscall.rs consumes them from there); deliver_notes + the note-action consts homed in syscall.rs (trap.rs's IRQ path will reach it as crate::syscall::deliver_notes). Both documented in place.
- **Files modified:** ../r9/x86_64/src/{proc.rs,syscall.rs}
- **Commit:** r9@1b2f023, r9@7132059

**5. [Rule 3 - Verify-command re-key] `cargo test -p x86_64 -- --nocapture` and bare check/clippy/boot-test cannot run as written (the 14-01..03 class)**
- **Found during:** every task's verify
- **Issue:** the crate's default target is no_std; this host has no bare cargo outside the nix devshell; boot-test needs R9_HARVEY_DIR.
- **Fix:** `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` inside `nix develop --command sh -c '...'`; check/clippy the same way; `R9_HARVEY_DIR=/home/req/code/harvey` exported for every boot-test. Same commands, same exit semantics.
- **Files modified:** none (execution-only re-key)
- **Commit:** n/a

**Total deviations:** 5 auto-fixed (2 blocking-infrastructure, 1 policy, 1 placement, 1 verify-environment). **Impact:** none on scope or guarantees — every plan verify leg ran and passed, and the eager-text deviation is recorded in the module doc and WINDOWS rather than left implicit.

## Authentication Gates

None.

## Known Stubs

None in the shipped code — but two honest era-limits are recorded in `.planning/WINDOWS.md` for ship-gate visibility:
- no trap-context pager on x86_64 (exec maps text/data/bss eagerly; deeper stack/tos growth beyond the two seed pages would fault-kill the proc — the pager is the follow-up)
- NSEC returns the loud no-RTC errstr until an epoch clock exists (the seam is in place)

## Issues Encountered

None beyond the documented deviations.

## TDD Gate Compliance

Task 2 carried `tdd="true"` and ran as a full RED→GREEN cycle (plan `type: execute`, `workflow.tdd_mode` false, so the classifier gate was not armed — evidence recorded here and in the commit messages):
- **RED** r9@6977748: 6 tests against degenerate stubs — `0 passed; 6 failed`, every failure on its planned assertion.
- **GREEN** r9@7132059: `6 passed; 0 failed` (58 total host suite).
- **REFACTOR:** assessed; no cleanup warranted (a faithful port; the shared disciplines live in the usermem constructors) — no refactor commit.

## Next Plan Readiness

Ready for 14-05 — the runtime gate has everything it needs: processes on the recorded address-space model, the armed entry calling a real dispatch, exec_into seeding frames 14-05's bootstrap consumes, and the dev_exec tick arms (timeout sweep + preempt) live behind the feature flag its config will set.

## Self-Check: PASSED

- r9 files exist: x86_64/src/{proc.rs, syscall.rs, swtch.rs} created; {exec.rs, trap.rs, dat.rs, seg.rs, pagealloc.rs, main.rs, platform/q35/timer.rs, l.S, Cargo.toml} modified in r9@abee15c
- r9 commits are ancestors of HEAD: 1b2f023, 6977748, 7132059, abee15c
- all task acceptance-criteria greps re-run PASS (logged in coverage)
- plan-level verification green: q35 ×2 byte-identical, q35net, ureg-check + negative control, aarch64 virt, check/clippy A/B-equal, 58+101 tests, aarch64+port diff empty
