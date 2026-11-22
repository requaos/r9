---
phase: 04-r9-userspace-exec-harvey-r9-abi
plan: 06
subsystem: kernel/syscall
tags: [r9, aarch64, syscall-dispatch, exec, devcons, devroot, sc2-gate, cross-repo-abi]

requires:
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 01)
    provides: D-14 measured syscall set (04-01-d14-trace.txt: 51 pwrite, 14 open, 50 pread, 4 close, 8 exits — no brk_/segbrk), initrd delivery (loader + patched DTB)
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 02)
    provides: port::paqfs no_std reader (devroot read_at backing)
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 03)
    provides: TrapFrame ≡ Ureg published ABI (number R0, args SP+8, result R0; ureg-check gate)
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 04)
    provides: proc/exec/fault machinery (fd_table seam, segments, mapping_log, EC split)
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 05)
    provides: first EL0 entry (trap_return), devroot mount + /init exec, tick-print cfg gating, EC-0x15 placeholder (replaced here)
provides:
  - aarch64/src/syscall.rs — SVC dispatch: [Option<fn>; 55] over the frozen sys.h, the measured set {PREAD(50), PWRITE(51), OPEN(14), CLOSE(4), EXITS(8), EXEC(7)}, loud-unimplemented default, T-04-11 per-access user-VA validation
  - devcons device for EL0 fds — fd 0/1/2 preopened (D-15), byte-verbatim write via Console MCS lock + PL011 polled TX, RX-ring read (cfg(dev_exec)); echo preserved under cfg(not(dev_exec))
  - EXEC handler (exec::exec_replace) — parse-first/free-second/seed-third image replacement + live-frame rewrite (T-04-13)
  - xtask boot-test exec profile (Banner → ExecEntry → InitPrint → HelloPrint → AllExited) + always-on negative scan
  - SC#2 SATISFIED: committed hello.7.out prints on r9 under qemu virt via the real EL0 syscall path; evidence log committed (04-06-sc2-boot.log)
  - pagealloc::deallocate_physpage + proc free paths (EXITS/EXEC resource reclamation)
  - dat.rs frozen open-mode constants (deferred-items.md #1 resolved)
affects:
  - 04-07 (scheduler: EXITS's all-exited halt becomes a reschedule; PROC_TABLE iteration + per-proc kstacks consumed)
  - 04-08 (notes at the trap_return boundary; NOTIFY/NOTED join the dispatch table)
  - Phase 5 (nested demand-paging from syscall context — deferred-items.md #3)

actuals:
  tokens: 36516
  tasks: 2
  commits: 4
  commits_r9: 3
  commits_harvey: 1
  plan_head_before_r9: 0598716d26bb5a5962dcfa76e10962af8782a74c
  plan_head_after_r9: 230fc7486a3d88fdbd2058068319e8645f731899
  plan_head_before_harvey: 3a75dc52a0e6673789eeb461d2870f266ec02733

tech-stack:
  added: []
  patterns:
    - "syscall handler contract: fn(&mut TrapFrame, &mut Proc) -> u64 — number from frame.x0, args validated + read from user SP+8, result to frame.x0; EXEC returns the seeded tos_va so dispatch's writeback agrees with the rewritten frame by construction"
    - "user-string egress: copy_user_str validates each byte's segment membership BEFORE reading, ERRMAX-bounded; EXEC copies path+argv to kernel memory before the old image's pages are freed"
    - "device-index routing on Chan.dev (DEV_ROOT=0 / DEV_CONS=1) until the generated devtab era; open modes are the frozen Plan 9 access-field values"
    - "exec image replacement ordering (T-04-13): parse+precheck → free (unmap leaf, THEN dealloc phys) → seed → rewrite the live frame; failure before the free leaves the old image intact"
    - "boot-test profile per config dev list (panic/exec/ticks-default), plus an always-on negative scan (panic|Unhandled interrupt fails any non-panic profile, checked per iteration and at finish)"

key-files:
  created:
    - ../r9/aarch64/src/syscall.rs
    - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-06-sc2-boot.log
  modified:
    - ../r9/aarch64/src/trap.rs (EC 0x15 → syscall::dispatch + return; frame x0/sp pub(crate); INTID-33 → rx_irq())
    - ../r9/aarch64/src/exec.rs (read_aout/install_image refactor + exec_replace + check_seed_fit)
    - ../r9/aarch64/src/proc.rs (fd 0/1/2 devcons preopen, validate_user_range, set_errstr, fd(), kstack_pas recording, free_user_mappings, free_root_table_and_kstack, any_active_procs)
    - ../r9/aarch64/src/fault.rs (Rule-1: page-in clamps the file read to the exact segment content end)
    - ../r9/aarch64/src/pagealloc.rs (deallocate_physpage)
    - ../r9/aarch64/src/main.rs (mod syscall)
    - ../r9/aarch64/src/platform/virt/devcons.rs (cons_write/cons_read device entry points)
    - ../r9/aarch64/src/platform/virt/pl011.rs (cfg(dev_exec) RX ring + cons_rx_drain; echo kept under cfg(not(dev_exec)))
    - ../r9/aarch64/src/platform/raspi3b/devcons.rs (unreachable stubs keeping shared syscall.rs compiling on raspi)
    - ../r9/port/src/dat.rs (DEV_ROOT/DEV_CONS + frozen O* open-mode constants replace the wrong Mode bitflags)
    - ../r9/port/src/devcons.rs (write_bytes — the byte-verbatim device write under the CONS lock)
    - ../r9/port/src/devroot.rs (root() accessor + image() getter + dat constants)
    - ../r9/xtask/src/main.rs (exec-profile BootStages + markers + negative scan + BootProfile + fail_pretty + 2 unit tests)
    - ../r9/HowItWorks.md (syscall dispatch + EXEC section — file-header mandate)
    - build/aarch64/init.c (print → exec("/hello", argv0) → loud failure path)
    - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md (#1 resolved, #3 added)

key-decisions:
  - "Implemented set = the 04-01 trace {pwrite(51), open(14), pread(50), close(4), exits(8)} ∪ {exec(7)} exactly — the trace lists no brk_(24)/segbrk(12), so they are NOT implemented (the D-14 conditional settled by evidence); every other number is loud-unimplemented + -1"
  - "devcons device writes are byte-verbatim (no kernel \\n-cooking) through the Console MCS lock — user-visible markers reach serial only via the EL0 binary's own pwrite path (the evidence-honesty prohibition is mechanical: the marker string is asserted present in the binary's own rodata)"
  - "EXEC's success path returns tos_va from the handler so dispatch's frame.x0 writeback IS the seeded x0 — keeps the plan's Option<fn(...) -> u64> table shape with no NOTED-style special case"
  - "open('#c/pid') fails soft (-1 + errstr) per the plan's named device set (cons + devroot only); getpid() tolerates the failure (atol of zeroed buffer) and exits still reaches _exits — matching the measured binaries' actual needs"
  - "EXITS frees user pages + root-table page + kstack pages (kstack LAST — the handler executes on it until the park); kernel VA aliases deliberately not torn down (nothing allocates after the all-exited halt this era)"

patterns-established:
  - "chan.dev match-dispatch in fd handlers (DEV_CONS device call / DEV_ROOT paqfs read_at / unknown → errstr + -1)"
  - "exec-profile evidence chain: banner → initrd → devroot open → exec entry → init print → hello print → process N exited → all user processes exited"

requirements-completed: [R9-02]

coverage:
  - id: T1
    description: "syscall.rs dispatch core: [Option<fn>; 55] with None → loud 'syscall N unimplemented' + -1; number from frame.x0, args at frame.sp+8 validated per-access via proc.validate_user_range (T-04-11); PWRITE offset -1 streams; OPEN resolves devroot names + cons/#c/cons; fd 0/1/2 preopened devcons chans at new_user_proc; tick println cfg(not(dev_exec))-gated; Phase-3 virt (ticks+echo) + virtpanic profiles green"
    requirement: "R9-02"
    verification:
      - kind: integration
        ref: "four-config builds (virtexec/default-raspi3b/raspi4b) + clippy green; plan grep chain (frame.sp + 8, validate_user_range, unimplemented, frame.x0, tick cfg gate, syscall::dispatch in trap.rs) PASS; boot-test virt PASS (banner + 3 ticks + r9echo7) + virtpanic PASS; EXTRA beyond plan: raw-qemu virtexec smoke (target/boot-logs/virtexec-t1-smoke.log) — init printed 'init: r9 EL0 alive' via pwrite(51), open(#c/pid) failed soft, EXITS(8) freed + printed 'process 1 exited: \"\"' + 'all user processes exited', zero panic/Unhandled lines"
      status: pass
    human_judgment: false
  - id: T2-SC2-GATE
    description: "SC#2: the committed Phase-2 hello.7.out (sha256 1c89e0990e386ae503d9f513d3a6c1985222f772d1edb40016c19aa3881abb04, asserted in the verify chain) executes on r9 via init's EXEC and prints 'hello aarch64 harvey' from the real EL0 syscall path; EXEC parses/validates the a.out before freeing the old image; marker string asserted in the binary's own rodata; zero panic/Unhandled-interrupt lines; evidence consolidated at 04-06-sc2-boot.log; Phase-3 profiles + raspi builds green"
    requirement: "R9-02"
    verification:
      - kind: integration
        ref: "stage-build.sh PASS (init rebuilt with exec; hello sha256 match); cargo xtask initrd (bootfs.paq sha256 1148a4f8284be9345d22a17348d272c0c8d74fc6635b8a6dd992e1b6897d6b85); cargo xtask boot-test --config virtexec PASS (ordered chain banner → exec: entering EL0 pc=0x10068 → init: r9 EL0 alive → hello aarch64 harvey → all user processes exited, ureg-check precondition green); marker greps PASS incl. 'init: exec returned' count 0 and panic/Unhandled count 0; strings(target/initrd-tree/hello) contains the marker (rodata honesty); boot-test virt + virtpanic re-PASS; raspi3b/raspi4b builds green; port 12/12 + aarch64 host 8/8 + xtask 19/19 tests"
      status: pass
    human_judgment: false

duration: 27min
completed: 2026-11-02
status: complete
---

# Phase 04 Plan 06: Syscall Dispatch + SC#2 Execution Gate Summary

**The Harvey↔r9 ABI is now a running system: SVC dispatch against the frozen wire ABI (number in R0, args at SP+8, result in R0) with exactly the D-14 measured handler set {PREAD, PWRITE, OPEN, CLOSE, EXITS} plus EXEC behind a per-access-validated user-pointer boundary — and the SC#2 gate satisfied with execution evidence: Phase 2's committed `hello.7.out` (sha256 `1c89e099…`) was exec'd by the staged init on r9 under qemu virt and printed `hello aarch64 harvey` from the real EL0 syscall path, followed by a clean all-exited halt. The gate caught and fixed one real 04-04 latent bug (the demand-pager's file-read clamp), and deferred-items #1 (dat.rs Mode skeleton) is resolved along the way.**

## Performance

- **Duration:** 27min
- **Started:** 2026-11-02T14:04:33Z
- **Completed:** 2026-11-02
- **Tasks:** 2/2
- **Commits:** 3 r9 + 1 Harvey (+ the metadata commit carrying this file)
- **Verification runs:** 4-config builds + clippy (×2 states), boot-tests virt/virtpanic/virtexec (multiple), port 12/12 + aarch64 8/8 + xtask 19/19 host tests, ureg-check green inside every boot-test

## Accomplishments

- **Task 1 (r9 `a19e649`) — the real syscall path exists and is runtime-proven.** `aarch64/src/syscall.rs` carries the `static DISPATCH: [Option<fn(&mut TrapFrame, &mut Proc) -> u64>; 55]` over Harvey's frozen sys.h 0–54 — the implemented set is exactly the 04-01 measured trace {pwrite(51), open(14), pread(50), close(4), exits(8)} (the trace carries no brk_(24)/segbrk(12), so the conditional inclusion does NOT fire — zero guessed syscalls); a None entry prints `syscall N unimplemented` and returns -1, loud never wedged. `trap()`'s EC 0x15 branch now calls `syscall::dispatch(frame)` and RETURNS (the placeholder park is gone) — the result written to `frame.x0` rides the shared `trap_return` tail's `ldp x0` pop into user R0. Every user-VA argument (the Sargs block, buffers, path strings) is validated against the proc's segment map BEFORE any kernel read/write (T-04-11); invalid → errstr + -1. fd 0/1/2 are devcons chans preopened at proc construction (D-15 seam filled); PWRITE offset -1 streams; devcons writes go byte-verbatim through the Console MCS lock + PL011 polled TX (so acceptance markers originate from the EL0 binary's own pwrite, never a kernel print on its behalf); the INTID-33 handler feeds an RX ring under cfg(dev_exec) and keeps echoing under cfg(not(dev_exec)) (the virt profile's r9echo7 assertion still passes). OPEN resolves devroot names (component-wise walk) plus the literal `cons`/`#c/cons`; EXITS frees the proc's pages (user mappings via mapping_log, root-table page, kstack pages via the construction-recorded PAs), marks Moribund, prints `process N exited: "msg"`, and prints `all user processes exited` + parks when the table drains. dat.rs's wrong-valued Mode bitflags are replaced with the frozen Plan 9 open-mode constants (deferred-items #1 resolved).
- **Task 2 (r9 `69853e6`, docs `230fc74`; Harvey `a8ab10cec`) — SC#2 SATISFIED.** `exec::exec_replace` implements donor sysexec semantics behind a running process: PARSE the new a.out + seed-fit precheck first (failure → -1/errstr, old image intact — T-04-13), FREE the old user mappings second (each leaf entry cleared before its phys page is deallocated), SEED third, then REWRITE THE LIVE TRAP FRAME in place (R0=tos, SP→new seed stack, PC=new entry, PSR=0, LR=0); the handler returns tos_va so dispatch's `frame.x0` writeback agrees with the seeded x0 by construction. The staged `init` (`build/aarch64/init.c`) now prints, execs `/hello`, and would loudly print + `exits("exec-fail")` on failure. The xtask boot-test gains the exec profile (dev list contains "exec"): Banner → `exec: entering EL0 pc=0x` → `init: r9 EL0 alive` → `hello aarch64 harvey` → `all user processes exited`, plus an always-on negative scan (any `panic`/`Unhandled interrupt` line fails a non-panic profile, per-iteration and at finish). **The evidence (04-06-sc2-boot.log, committed):** banner → initrd → paqfs entries → `devroot: /init open (37344 bytes)` → `exec: entering EL0 pc=0x10068 sp=0x7fffffffffd8` → TTBR0 switch → demand page-ins → init's print → exec → hello's print → `process 1 exited: ""` → `all user processes exited`; zero panic/Unhandled lines; the marker string is asserted present in the staged binary's own rodata; hello is byte-identical to the committed ABI bisect binary.
- **The gate caught a real latent bug (fixed inline, Rule 1).** The first full-length run died when `exits()`'s atexit loop made an indirect call to `0x732e736174` — the ASCII bytes `"tas.s"`. Root cause in 04-04's `page_in_from_image`: the file read was NOT clamped to the segment's content end, and the paqfs entry's length spans the whole file (syms included), so trailing syms bytes bled into the bss window of the last data page — `onex[i].f` read `"tas.s"` as a function pointer. The demand-pager now clamps every text/data read to the exact content end (pages stay zero-prefilled past it). This would have poisoned ANY libc binary's exit path; the 04-05 gate never ran long enough to touch it.

## Task Commits

| Task | Repo | Commit | Files |
|------|------|--------|-------|
| 1 | r9 | `a19e649` | syscall.rs (new), trap.rs, proc.rs, fault.rs, pagealloc.rs, main.rs, virt/devcons.rs, virt/pl011.rs, raspi3b/devcons.rs, port dat.rs/devcons.rs/devroot.rs |
| 2 | r9 | `69853e6` | syscall.rs (EXEC), exec.rs (exec_replace), port/devroot.rs (image()), xtask/main.rs (exec profile + negative scan + tests) |
| 2 (docs) | r9 | `230fc74` | HowItWorks.md (file-header mandate) |
| 2 | harvey | `a8ab10cec` | build/aarch64/init.c, 04-06-sc2-boot.log (the SC#2 evidence) |
| metadata | harvey | (final commit) | .planning SUMMARY/STATE/ROADMAP/REQUIREMENTS/deferred-items |

**r9 HEAD at evidence time (D-13 ordering — all r9 commits landed first):** `230fc7486a3d88fdbd2058068319e8645f731899` (docs-only on top of the gate-producing code commit `69853e6`; the gate ran on the exact committed tree).
**Evidence-run bootfs.paq sha256:** `1148a4f8284be9345d22a17348d272c0c8d74fc6635b8a6dd992e1b6897d6b85` (per-bake digest — mkpaqfs embeds a time field).
**hello sha256 (asserted in the verify chain):** `1c89e0990e386ae503d9f513d3a6c1985222f772d1edb40016c19aa3881abb04` (unchanged, the committed Phase-2 bisect binary).

## Files Created/Modified

**r9:** `aarch64/src/syscall.rs` (new, ~330 lines — table + dispatch + 6 handlers + copy_user_str), `aarch64/src/trap.rs` (EC 0x15 routing, x0/sp pub(crate), rx_irq), `aarch64/src/exec.rs` (read_aout/install_image refactor + exec_replace + check_seed_fit), `aarch64/src/proc.rs` (preopen, validate_user_range, set_errstr, fd, kstack_pas, free paths, any_active_procs), `aarch64/src/fault.rs` (read clamp), `aarch64/src/pagealloc.rs` (deallocate_physpage), `aarch64/src/main.rs` (mod syscall), `aarch64/src/platform/virt/devcons.rs` (cons_write/cons_read), `aarch64/src/platform/virt/pl011.rs` (RX ring under dev_exec), `aarch64/src/platform/raspi3b/devcons.rs` (stubs), `port/src/dat.rs` (DEV_* + frozen open modes), `port/src/devcons.rs` (write_bytes), `port/src/devroot.rs` (root()/image()), `xtask/src/main.rs` (exec profile + negative scan), `HowItWorks.md`.

**Harvey:** `build/aarch64/init.c` (the 04-06 staged init), `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-06-sc2-boot.log` (committed evidence), `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md`.

## Decisions Made

See key-decisions in the frontmatter: the exact implemented set (trace ∪ EXEC, nothing else), byte-verbatim device writes (evidence honesty), EXEC's tos_va return convention, the soft-fail `#c/pid` open, and the EXITS free ordering.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug in 04-04's demand pager, exposed by this plan's longer runtime] page-in file read not clamped to the segment content end**
- **Found during:** Task 1 post-verify extra smoke boot (virtexec): after `init: r9 EL0 alive` + open(#c/pid), the process died on an EL0 fetch fault at `0x732e736174` (`"tas.s"` as ASCII) — exits()'s atexit loop read a poisoned `onex[i].f` and made an indirect call into syms bytes
- **Issue:** `page_in_from_image` read the full 4 KiB page from the paqfs entry, whose `length` covers the WHOLE file (header+text+data+syms); the last text/data pages' tails — which must stay zero (padding/bss) — received trailing file bytes (the syms section)
- **Fix:** clamp the read to the exact content end (text: `TEXT_MAP_BASE + data_file_base`; data: the exact `data.end()`); pages remain zero-prefilled past the content
- **Files modified:** ../r9/aarch64/src/fault.rs
- **Verification:** full runtime chain passes (init print → open soft-fail → EXITS → all-exited); re-verified at the final committed state with the exec profile's SC#2 chain
- **Commit:** a19e649 (r9)

**2. [Rule 1 — Acceptance-letter] Task-2 leg-1's hello sha256 test could never pass as written**
- **Found during:** Task 2 verify leg 1
- **Issue:** `test "$(sha256sum F 2>/dev/null || shasum -a 256 F | awk '{print $1}')" = "HASH"` — sha256sum's output is `HASH  F` (hash + filename), so the bare-equality fails even on a match (the shasum fallback arm shows the intent: hash-only)
- **Fix:** ran the intent-equivalent — `sha256sum build/aarch64/out/hello | awk '{print $1}'` compared to the pinned digest (PASS)
- **Files modified:** none (verification invocation only)

**Plan-supporting additions (in-plan intent):** raspi3b/devcons.rs cons_read/cons_write unreachable stubs (the plan names only the virt file; the platform-shared syscall.rs must compile on raspi for the four-config gate — the stubs are unreachable there: no EL0 without dev_exec); pagealloc::deallocate_physpage wrapper (the plan's "free pages via mapping_log" requires an allocator free — bitmapalloc had deallocate, pagealloc had no wrapper); proc.rs kstack PA recording (the plan's "free the kstack" needs PAs the bump-VA allocation never retained); devroot::root()/image() accessors (the plan's "devroot walk for OPEN / devroot OEXEC for EXEC" from syscall context needs the mounted root tree); dat.rs DEV_ROOT/DEV_CONS + frozen open-mode constants (chan.dev routing for the plan's handlers; resolves deferred-items #1 which named 04-06 as its fixer); xtask BootProfile enum + fail_pretty (the always-on negative scan needed a stage-less fail path at finish()).

**Total deviations:** 2 auto-fixed + 6 plan-supporting additions. No Rule 4 (architectural) items.
**Impact on plan:** none to intent — every must-have truth, acceptance criterion, and plan-level verification item passes at the final state.

## Known Stubs

None introduced by this plan — the EC-0x15 placeholder is gone (real dispatch), the fd 0/1/2 seam is filled (devcons preopen), and PREAD's devcons-read path is real (returns 0 on empty per plan — the no-blocking-console-read scope note, not a stub). The EXEC handler's argv caps (32/128) are validation bounds per plan; the seed-page fit precheck keeps over-capacity execs atomic (fail before free). The remaining note/scheduler seams (note_queue, notify_handler, wait_record, context) belong to 04-07/04-08 per their plans. WINDOWS.md note: the deviations above are fixed (nothing open); cross-repo (`../r9/…`) paths remain unrecordable in the ledger, matching 04-04/04-05's note — recorded here instead.

## Issues Encountered

- The stale-artifact boot trap: `cargo xtask boot-test --config virtpanic` rebuilt `target/aarch64-unknown-none-elf/debug/aarch64` (a shared output path), so my first raw-qemu virtexec smoke booted the *panic* build. Not a code bug; rebuilt virtexec before re-running. (Evidence-freshness rule stands: every boot-test invocation builds its own image — only manual raw-qemu runs need the discipline.)
- The Rule-1 clamp bug cost three instrumented boot iterations to pin (syscall trace → frame+stack dump → byte-pattern decode); the fix verified first-try after the root cause was identified.

## Threat Flags

None. All three register mitigations landed as designed: T-04-11 (per-access validation of every user-VA argument — Sargs block, buffers, path strings, argv array + each pointed-to string — via `Proc::validate_user_range` before any kernel read/write), T-04-12 (the tick print stays `cfg(not(dev_exec))`-gated — no IRQ-context code ever takes the Console MCS lock once syscalls can print; the PL011 RX ring is lock-free single-writer/single-reader with the reader IRQ-masked), T-04-13 (parse+precheck before free; failure leaves the old image intact; the frame rewrite happens only after a fully seeded new image). T-04-SC: no package installs this plan (no new crates). The one newly-surfaced boundary note — segment-level validation vs. not-yet-faulted pages (loud EL1h abort, never silent) — is logged as deferred-items.md #3.

## User Setup Required

None.

## Next Phase Readiness

- **04-07 (scheduler):** EXITS's all-exited halt is the named reschedule point; PROC_TABLE iteration (`any_active_procs`), per-proc kstacks + recorded PAs, the swtch Context slots, and EL0-origin IRQ dispatch (already returning to userland correctly) are all in place. The tick's re-arm is live in every config — turning it into a reschedule is a timer.rs call-site change.
- **04-08 (notes):** NOTIFY(28)/NOTED(29) join the dispatch table (the NOTED no-result-writeback special case will need the table's return convention revisited — the EXEC tos_va trick shows the shape); note delivery rewrites the frame at the `trap_return` boundary; devproc-lite exposes only `/proc/N/note` (devroot's walk/open/read + the DEV_* index convention are the template).
- **Dispatch ergonomics for the next handlers:** `sargs(frame, proc, arity)` + `copy_user_str` + `err(proc, msg)` are the reusable pieces; adding a handler is one table entry + one fn.
- The initrd iteration loop is unchanged: `sh build/aarch64/stage-build.sh && (cd ../r9 && nix develop -c cargo xtask initrd)` — userland changes never relink the kernel (D-01/D-03).
- r9 HEAD for downstream pinning (D-13): `230fc7486a3d88fdbd2058068319e8645f731899`.
- Flagged assumption carried from the plan: R9-02 remains probe-unclassified — acceptance derived from ROADMAP SC#2 + CONTEXT D-14/D-15 and the 04-01 trace artifact; the verifier should treat the probe row as a flagged assumption, not a resolved edge.

## Self-Check: PASSED

- Key files exist: r9 `aarch64/src/syscall.rs` ✓ `exec.rs` exec_replace ✓ `xtask/src/main.rs` exec profile ✓ `HowItWorks.md` section ✓; Harvey `build/aarch64/init.c` ✓ `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-06-sc2-boot.log` ✓ (committed, 469 lines).
- Commits: r9 `git log --grep="04-06"` → a19e649 (Task 1), 69853e6 (Task 2), 230fc74 (docs) — ≥1 `04-06` commit ✓; Harvey → a8ab10cec (init.c + evidence log) ✓, plus the metadata commit carrying this file.
- Acceptance criteria re-run at close-out at final HEADs: Task-1 legs (four-config builds + clippy green; grep chain frame.sp + 8 / validate_user_range / unimplemented / frame.x0 / tick cfg gate / syscall::dispatch PASS; boot-test virt PASS with ≥3 ticks + r9echo7; virtpanic PASS) ✓; Task-2 legs (stage-build PASS + hello sha256 match; initrd bake; boot-test virtexec PASS; marker greps incl. 'init: exec returned' count 0 + panic/Unhandled count 0; strings rodata-honesty PASS; evidence copy committed; virt + virtpanic re-PASS; raspi3b/raspi4b builds green) ✓.
- Plan-level verification at final HEADs: SC#2 satisfied (ordered chain + rodata cross-check + zero panic lines, evidence committed); SC#1 ureg-check ran green ahead of every boot-test profile; regression fan (virt/virtpanic/raspi3b/raspi4b/clippy) green; port 12/12, aarch64 host 8/8, xtask 19/19.

---
*Phase: 04-r9-userspace-exec-harvey-r9-abi*
*Completed: 2026-11-02*
