---
phase: 14-harvey-amd64-userspace-on-r9
plan: 02
subsystem: syscall-entry
tags: [syscall, entry, msr, x86_64, r9, naked-asm, xexec-02]
duration: 11 min
completed: 2026-11-08
requires:
  - "Phase-14-01-complete r9 HEAD 0f01476 (the 184-byte frozen Ureg, amd64/ABI.md, dual-arch ureg-check gate)"
provides:
  - "the armed SYSCALL/SYSRET entry contract: Star/Lstar/Sfmask + the GS-base pair written at kernel init in the vsvm.c:174-185 donor order"
  - "the syscallentry/syscallreturn/touser naked-asm trio in r9 trap.rs — the software-built hardware frame on the frozen 184-byte Ureg, donor-exact arithmetic"
  - "the fillable dispatch-stub shape (fn syscall(number, frame)) 14-04's syscall module plugs into without signature change"
affects:
  - "../r9/x86_64/src/{seg,cpu,dat,trap,main}.rs"
  - "14-04 (the dispatch body), 14-05 (the touser handoff + per-proc rsp0), every Phase-14 ring-3 plan"
tech-stack:
  added: []
  patterns:
    - "gs-relative SYSCALL entry with zero GP carrier registers (the Mach scratch word + TSS rsp0 replace the donor's RMACH/RUSER sacrifice)"
    - "software-built hardware frame on a frozen ABI frame (SYSCALL pushes nothing; the entry pushes ss/sp/flags/cs/ip itself)"
key-files:
  created: []
  modified:
    - ../r9/x86_64/src/seg.rs
    - ../r9/x86_64/src/cpu.rs
    - ../r9/x86_64/src/dat.rs
    - ../r9/x86_64/src/trap.rs
    - ../r9/x86_64/src/main.rs
key-decisions:
  - "Lstar sequenced never-at-a-hole: Task 1's arming commit deferred the Lstar write, Task 2 armed it at the real entry in the same plan — per-task commits kept without ever pointing Lstar at a stub"
  - "the kernel-stack switch reads the TSS rsp0 gs-relative (the ONE mechanism both ring-3 entry paths share) instead of the donor's m->proc->kstack walk — r9's Proc is the 14-03 substrate and boot-time rsp0 names the Mach scheduler stack until 14-05 re-points it"
  - "all 15 GP slots hold TRUE user values (strictly better than the donor, which sacrifices r13/r14/r15 to carrier duty): the gs-relative switch needs no carrier register; the only scratch is the Mach's entry word, stashed/fetched with IF=0 (Sfmask=If makes the window atomic)"
  - "the GP half is stored/restored around the Rust dispatch (9k's C dispatch preserves everything-but-AX by 6c convention; Rust's ABI clobbers caller-saved regs freely) — frame arithmetic stays donor-exact"
requirements-completed: []   # XEXEC-02 is declared by 14-02/03/04/05 (shared-ID gate): completes with 14-05
commits: 1
plan_head_before: e5f9037b00cee9af6cf1fb4dc5d71f5a7761b5f2
plan_head_after: e5f9037b00cee9af6cf1fb4dc5d71f5a7761b5f2
actuals:
  tokens: 4970    # chars/4 over the realized r9 diff (0f01476..2b16f61: 405+/13-)
  tasks: 2
  commits: 2      # r9 repo (f44c29f, 2b16f61); the harvey-side docs commit follows this file
status: complete
coverage:
  - deliverable: "MSR arming revival (star/utextsel/fmask per the 9k donor) wired at kernel init (Task 1, tracer)"
    verification:
      - kind: command
        ref: "cargo xtask boot-test --arch x86-64 --config q35 (x2 consecutive), q35net — all PASS with arming live"
        status: pass
      - kind: command
        ref: "cargo xtask boot-test --arch aarch64 --config virt — PASS (sibling untouched)"
        status: pass
      - kind: command
        ref: "cargo xtask check + clippy exit 0; port tests 101, x86_64 bins host-target 26 — all green"
        status: pass
      - kind: command
        ref: "acceptance greps: star() main.rs=2 (>=1), fmask cpu.rs=3 (>=2), Lstar mentions=6 (>=2), aarch64+port diff empty"
        status: pass
    human_judgment: false
  - deliverable: "syscallentry + syscallreturn + touser naked asm on the frozen Ureg (Task 2)"
    verification:
      - kind: command
        ref: "cargo xtask build --arch x86-64 --config q35 exit 0 (the Lstar reference links — presence guard)"
        status: pass
      - kind: command
        ref: "cargo xtask boot-test q35 x2 + q35net PASS with the full trio armed-but-idle; aarch64 virt PASS"
        status: pass
      - kind: command
        ref: "acceptance greps: syscallentry=5 (>=2), touser=3 (>=2), syscallreturn=5 (>=1), swapgs-sites=35 (>=3)"
        status: pass
    human_judgment: false
---

# Phase 14 Plan 2: SYSCALL/SYSRET Entry Contract (MSR arming + the entry trio) Summary

Armed the 9k-donor SYSCALL entry contract at r9 kernel init (Star/Lstar/Sfmask + the GS-base pair, vsvm.c order) and landed the syscallentry/syscallreturn/touser naked-asm trio that software-builds the hardware frame SYSCALL never pushes onto the frozen 184-byte Ureg — every existing gate green with the path armed-but-idle.

**Duration:** 11 min (2026-11-08T19:23:38Z → 2026-11-08T19:34:09Z)
**Tasks:** 2/2 complete
**Files:** 5 (all r9-side modified; nothing created, nothing Harvey-side beyond this summary)

## Commits

| Repo | Hash | Message |
|------|------|---------|
| r9 | f44c29f | feat(14-02): arm the SYSCALL MSR contract at init (star/fmask revival, Task 1) |
| r9 | 2b16f61 | feat(14-02): the syscallentry/syscallreturn/touser entry trio on the frozen Ureg (Task 2) |

## Accomplishments

### Task 1: MSR arming revival wired at kernel init (tracer)

- **seg.rs:** `Gdt::star()`/`Gdt::utextsel()` revived from the dead code (the `#[allow(dead_code)]` retired) — the SYSRET +16/+8 selector math and its AMD-vs-Intel RPL comment were already correct for r9's GDT order (ktext@8/kdata@0x10/udata@0x18/utext@0x20); now consumed by the arming.
- **cpu.rs:** `fmask()` re-keyed to the donor's If-only value (`wrmsr(Sfmask, If)`, vsvm.c:185) — the dead code's IF|TF|DF deliberately NOT carried (D-02a: whatever 9k emits; TF/DF pass through per ABI.md's wire contract), deviation noted in the doc comment. Added the MSR const family (EFER/STAR/LSTAR/SFMASK/FS_BASE/GS_BASE + EFER_SCE); MSR_KERNEL_GS_BASE's transitional allow retired with the rest.
- **main.rs:** `arm_syscall_entry()` at kernel init right after the GDT/IDT/TSS load — the vsvm.c:174-185 donor order: FSbase=0, GSbase=Mach (idempotent with l.S's warp64 wrmsr), KernelGSbase=0 (what makes the return-path SWAPGS land the user on base 0), Efer read-modify-write |= Sce (preserving l.S's LME/NXE), Star=`Gdt::star()`, Sfmask=`fmask()`. The Lstar write deferred to Task 2's commit — never armed at a hole.
- **Discovery recorded in the arming comments:** l.S already sets Efer.Sce at boot with Star/Lstar/Sfmask at 0 — Phase 13 has been booting with SYSCALL enabled-but-unrouted; the arming adds the routing and Task 2's Lstar makes an accidental SYSCALL land at a real entry instead of address 0 (strictly safer; still unreachable — no ring-3 code exists).
- **Regression:** q35 ×2, q35net, aarch64 virt all PASS with arming live; check/clippy exit 0; port tests 101 green; x86_64 host-target bins 26 green.

### Task 2: the entry trio on the frozen Ureg

- **syscallentry (trap.rs):** SWAPGS unconditional (SYSCALL only fires from ring 3 — no CS-check witness; NMI-safe because intrspecial re-derives its swap from rdgsbase); the user sp stashed in the Mach's entry scratch word (`movq %rsp, %gs:SCRATCH` — IF=0 per Sfmask=If, the window is atomic); the kernel stack switched via `movq %gs:TSS_RSP0, %rsp` (the same rsp0 the CPU loads for ring-3 IDT delivery — one kernel-stack mechanism for both entry paths; boot-time rsp0 names the Mach scheduler stack until 14-05 re-points it per-proc); the hardware half software-built in the donor's push order (ss constructed selector, saved user sp, R11 flags, cs constructed selector, RCX ip); `subq $(18*8)` carve; the packed quartet stores at (15*8+0..6) with ds CONSTRUCTED as the flat user data selector and es/fs/gs live (donor-verbatim); all 15 GP slots stored with TRUE user values (the donor sacrifices r13/r14/r15 as RMACH/RUSER carriers — the gs-relative switch needs none); the dispatch args pushed donor-style (Ureg*, number-from-BP) and reloaded into the Rust convention's rdi/rsi with a one-qword SysV alignment pad; `callq syscall`; the args dropped and the donor's fall-through taken as a `jmp syscallreturn` (Rust naked fns cannot share a fall-through body).
- **syscallreturn:** ax (the result) and bp (the notify channel — syscall.c's notify may rewrite ureg->bp) re-read THROUGH the frame; the GP half restored (Rust's dispatch clobbered the caller-saved set); then the donor's post-ADDQ choreography — CLI, SWAPGS, the quartet loads at frame 120..126, CX=ip (144), R11=flags (160), the user sp loaded LAST (168 — the load destroys the frame pointer), SYSRETQ (CS/SS derive from STAR).
- **touser:** the donor's first-ring-3 entry — CLI, SWAPGS, the flat user data selector into DS/ES/FS/GS, CX=UTZERO+0x28 (0x200028, the seeded entry past the a.out header), R11=If (the user enters with IF=1 — the unmask-at-handoff discipline), BP→SP (RARG carries the user sp), SYSRETQ. Transitional `#[allow(dead_code)]` until 14-05's exec_bootstrap consumes it.
- **The dispatch stub:** `extern "C" fn syscall(number: u64, frame: &mut Ureg)` — prints loud and panics if ever reached (unreachable until a user image execs); the SHAPE is the contract 14-04 fills (number from BP/RARG, result into the frame's ax slot, returns normally so the tail runs after).
- **Witnesses:** trap.rs const-assert block binds the asm's slot literals to the frozen layout via the exported UREG_* offsets (pc==18*8, CS==19*8, BP==6*8, DS==15*8, SP==21*8, carve+hw halves==184) plus selector pins (USER_DATA_SEL==0x1b, USER_TEXT_SEL==0x23 catching any GdtData reorder); dat.rs grew the `UREG_BP/DS/SP_OFFSET` computed pub consts (the 14-01 UREG_TRAPNO/CS family extended) and `MACH_SCRATCH_OFFSET`; seg.rs grew `Gdt::udatasel()` and `TSS_RSP0_OFFSET`.
- **main.rs:** the Lstar write lands (`wrmsr(MSR_LSTAR, syscallentry)`) — the missing-symbol link error is the presence guard; Lstar is no longer 0.
- **Regression:** build q35 exit 0 (trio linked); q35 ×2 + q35net PASS armed-but-idle; aarch64 virt PASS; check/clippy green (x86_64 warnings unchanged at the pre-existing 9); port tests 101, x86_64 bins 26.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] clippy's deny-by-default `eq_op` rejected the literal-vs-literal offset witnesses**
- **Found during:** Task 2 verify (clippy leg)
- **Issue:** the planned "offset-constant asserts against the frozen layout" written as `assert!(6*8 == 48)` fail `cargo xtask clippy` — clippy const-folds both operands and denies the comparison as equal-expressions (`assertions_on_constants` warns alongside).
- **Fix:** extended 14-01's exported `UREG_*` offset-const family in dat.rs with computed `UREG_BP_OFFSET`/`UREG_DS_OFFSET`/`UREG_SP_OFFSET` (path-vs-literal asserts are the shape dat.rs itself uses and clippy accepts) and asserted THOSE against the asm's literals — a strictly stronger binding (asm literal ↔ struct, not asm literal ↔ restated table number).
- **Files modified:** ../r9/x86_64/src/dat.rs, ../r9/x86_64/src/trap.rs
- **Commit:** r9@2b16f61

**2. [Rule 2 - Missing-critical] `Gdt::udatasel()` + `TSS_RSP0_OFFSET` + `MACH_SCRATCH_OFFSET` did not exist**
- **Found during:** Task 2 implementation
- **Issue:** the entry asm needs the flat user DATA selector (the donor's SSEL(SiUDS, SsRPL3) — for the ss push, the ds store, touser's four loads) and the Mach-relative rsp0/scratch offsets; seg.rs only had ktextsel/tasksel/utextsel and the fields are private across modules.
- **Fix:** `Gdt::udatasel()` (raw GDT offset, RPL ORed at use sites — consistent with the existing getters), `TSS_RSP0_OFFSET` (in-module offset_of in seg.rs), `MACH_SCRATCH_OFFSET` (in-module offset_of exported from dat.rs) — all computed, never hand-pinned.
- **Files modified:** ../r9/x86_64/src/seg.rs, ../r9/x86_64/src/dat.rs
- **Commit:** r9@2b16f61

**3. [Rule 3 - Verify-command re-key] `cargo test -p x86_64 -- --nocapture` and bare `cargo xtask check/clippy` cannot run as written (same class as 14-01's deviations)**
- **Found during:** Task 1/Task 2 verify
- **Issue:** the x86_64 crate's default target is no_std (host-target bin-test compile fails without `--bins --target x86_64-unknown-linux-gnu`); this host has no bare cargo outside the nix devshell; the boot-test precondition requires R9_HARVEY_DIR explicitly (05-CONTEXT D-10 — no sibling default).
- **Fix:** ran `cargo xtask test` (101) plus the explicit `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` (26); check/clippy inside `nix develop --command sh -c '...'`; `R9_HARVEY_DIR=/home/req/code/harvey` exported for every boot-test. Same commands, same exit semantics.
- **Files modified:** none (execution-only re-key)
- **Commit:** n/a

**4. [Rule 3 - Blocking] the broken-windows ledger was corrupted (pre-existing), blocking the mandated stub recording**
- **Found during:** SUMMARY close-out (`gsd_run windows append`)
- **Issue:** table rows 44/45 existed in the rendered table (out of id order, after row 50) but not in the fenced JSON block (the sole source of truth); every append failed the count check, and after reconciling the counts, the row-order drift guard.
- **Fix:** reconstructed the two JSON entries from their table rows (descriptions verbatim), inserted them in id order in the JSON block, and moved the two table rows to their renderTable position; the appends then succeeded and this plan's two stubs are recorded.
- **Files modified:** .planning/WINDOWS.md
- **Commit:** the plan's docs commit

**Total deviations:** 4 auto-fixed (1 clippy-blocker re-key, 1 missing-helper addition, 1 verify-environment re-key, 1 ledger-corruption repair). **Impact:** none on scope or guarantees — the witness set is stronger than planned, every verify leg ran and passed, and the ship-gate ledger is writable again.

## Authentication Gates

None.

## Known Stubs

| Stub | File | Line | Resolution |
|------|------|------|------------|
| the syscall dispatch body prints + panics instead of dispatching | ../r9/x86_64/src/trap.rs | the `extern "C" fn syscall` stub | INTENTIONAL and plan-declared ("a fillable stub because 14-04 replaces its body, not its shape"); unreachable until 14-05 execs a user image (armed-but-idle) |
| touser is dead code (no caller) | ../r9/x86_64/src/trap.rs | the `touser` naked fn | plan-intended: 14-05's exec_bootstrap is the handoff consumer; transitional `#[allow(dead_code)]` |

Both are recorded in `.planning/WINDOWS.md` (broken-windows ledger) for ship-gate visibility.

## Issues Encountered

None beyond the documented deviations.

## TDD Gate Compliance

N/A — this plan is `type: execute` (no tdd="true" tasks; the const-assert witnesses are plan content, not TDD cycles).

## Next Plan Readiness

Ready for 14-03 (the exec substrate: Proc, usermem, exec_into) — the entry contract its processes will SYSCALL through is armed and proven inert; 14-04's dispatch has its fillable shape; 14-05's handoff has touser and the per-proc rsp0 mechanism (Tss::set_rsp0) to key.

## Self-Check: PASSED

- r9 files exist: seg.rs/cpu.rs/dat.rs/trap.rs/main.rs all present and modified in r9@2b16f61
- commits are ancestors of r9 HEAD: f44c29f ✓, 2b16f61 ✓
- all task acceptance-criteria greps re-run PASS (logged in coverage)
- plan-level verification blocks green (q35 ×2, q35net, virt, build, check, clippy, tests)
