---
phase: 261104-b4p
plan: 01
subsystem: kernel-lifecycle
tags: [r9, devcons, echo, devmnt, fid, sys_sleep, exit-sweep, mk, bquote, flake, aarch64, plan9]

requires:
  - phase: 05-shell-boot-one-command-integration
    provides: the deferred ledger, the gate drivers, the boot-test fan
provides:
  - cooked-mode console echo (typed bytes on the wire, gated marker discipline)
  - #c/pid host pins + the pid-warning-absent gate stage
  - exit_tail's last-sharer fd-group sweep (fdgrp drain_all)
  - note-interruptible sys_sleep (donor tsleep) + smokesleep e2e proof
  - Arc'd devmnt fids with exactly-once Tclunk + the namec walk sweep
  - Harvey mk nested-backquote parse fix (donor depth counter)
  - flake systems list of three (x86_64-darwin dropped)
  - both deferred ledgers cleared (8 resolutions + 2 evidence-only flips)
affects: [milestone-v1.0-tag, r9-pin-bump, ci]

actuals:
  tokens: 142920
  tasks: 3
  commits: 9

tech-stack:
  added: []
  patterns:
    - "post-lock echo emission (INPUT lock drops, then hooks.echo — the debug_dump_input precedent)"
    - "Arc'd fid + clunk_if_last: the close dispatch fires only on the last live ref, copen-independent for DEV_MNT"
    - "namec walk sweep: minted DEV_MNT intermediates clunk_if_last'd on EVERY exit (Ok and Err); the Arc count discriminates entry-dup/chain-clone/returned-chan"
    - "two-occurrence expect markers + strip-the-echoed-line sweep captures (the echo-era false-PASS defense)"
    - "e2e RED proof via kernel-revert patch-dance (working-tree diff saved, HEAD restored, gate fails on the absent marker, GREEN re-applied)"

key-files:
  created:
    - build/aarch64/smokesleep.c
    - build/aarch64/bquote-repro.sh
  modified:
    - ../r9/port/src/devcons.rs
    - ../r9/port/tests/devcons_test.rs
    - ../r9/aarch64/src/main.rs
    - ../r9/port/src/fdgrp.rs
    - ../r9/port/tests/fdgroup_test.rs
    - ../r9/port/src/dat.rs
    - ../r9/port/src/devmnt.rs
    - ../r9/port/src/namec.rs
    - ../r9/port/tests/devmnt_test.rs
    - ../r9/aarch64/src/syscall.rs
    - ../r9/aarch64/src/proc.rs
    - ../r9/xtask/src/main.rs
    - build/aarch64/drive-r9.expect
    - build/aarch64/smoke-r9.expect
    - build/aarch64/init.c
    - build/aarch64/stage-build.sh
    - sys/src/cmd/mk/lex.c
    - flake.nix
    - .planning/phases/05-shell-boot-one-command-integration/deferred-items.md
    - .planning/phases/02-aarch64-toolchain-libc-9front-donor/deferred-items.md

key-decisions:
  - "Item 5 LANDED (no escape hatch needed): the Arc'd fid + clunk_if_last + namec sweep held every gate on the first landing"
  - "Caller-side close sweeps beyond the plan's named set (sys_exec/sys_stat/sys_chdir/sys_open-err/sys_bind/sys_mount/sys_unmount targets) were REQUIRED for correctness: under open-shares-the-Arc, sys_exec's walked chan would otherwise strand the fid the value-copy era clunked exactly once — a per-command regression leak"
  - "The rawon-flush echo pin was shaped around the single-threaded test rig: a completed-then-stranded partial (abc\\ndef) exercises the flush without a parked read (rawon's mid-park flush is an era-limit non-event — no gate rawons mid-read)"
  - "Host-built mk evidence needed plan9port-era scratch-only fixups (p9waitfor rename, regexp9 union Resub, -std=gnu89, shell path) — the working tree's lex.c carries ONLY the 3-line donor depth counter"

patterns-established:
  - "Expect-driver echo discipline: every typed command's marker is staged twice (echoed, then output); sweep captures strip through the first newline before presence-asserting"
  - "Pid-file pin shape: separate set-once hook static, safe beside the scripted cons suite in one test process"

requirements-completed: [QUICK-261104-B4P]

coverage:
  - id: D1
    description: "Cooked console echo on the wire (typed bytes echo; BS-SP-BS on a real pop; CRLF completers; ^U/^D/DEL silent; raw mode never echoes; rawon flush never retro-echoes)"
    requirement: QUICK-261104-B4P
    verification:
      - kind: unit
        ref: "r9 port/tests/devcons_test.rs#cons_read_path_cooked_raw_del_and_consctl (the echo arms)"
        status: pass
      - kind: e2e
        ref: "R9_KERNEL=… timeout 900 expect build/aarch64/drive-r9.expect — TWO consecutive ALL-STAGES-PASS (build/aarch64/drive-r9.log)"
        status: pass
    human_judgment: false
  - id: D2
    description: "#c/pid pinned (decimal-ASCII read, offset tail, streaming-offset advance, OWRITE refusal, stat/listing length) + both drivers' pid-warning-absent stage"
    requirement: QUICK-261104-B4P
    verification:
      - kind: unit
        ref: "r9 port/tests/devcons_test.rs#pid_file_serves_the_callers_pid"
        status: pass
      - kind: e2e
        ref: "drive-r9 + smoke-r9 'pid-warning-absent' stages green (no #p/0/ctl in either capture)"
        status: pass
    human_judgment: false
  - id: D3
    description: "exit_tail sweeps the last-sharer's fd group (drain_all pinned; close dispatch never under the group lock; sharers/devsrv-clone/paqfs-child untouched)"
    requirement: QUICK-261104-B4P
    verification:
      - kind: unit
        ref: "r9 port/tests/fdgroup_test.rs#fd_group_drain_all_empties_every_slot"
        status: pass
      - kind: e2e
        ref: "five boot profiles + full smoke green (rc per-generation exits + init's restart loop regression-green)"
        status: pass
    human_judgment: false
  - id: D4
    description: "sys_sleep interrupts on a pending note (donor tsleep): smokesleep's alarm(300) interrupts sleep(30000) — 'smokesleep: interrupted' in all three exec-family profiles; expiry still returns 0"
    requirement: QUICK-261104-B4P
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch aarch64 --config {virtexec,virtmulti,virtnotes} — the AllExited smokesleep assertion (e2e RED proof: pre-fix kernel ran to expiry, marker absent)"
        status: pass
      - kind: e2e
        ref: "nix run .#boot-r9-smoke --override-input r9 git+file:///Users/neilskinner/code/r9 — restart loop green"
        status: pass
    human_judgment: false
  - id: D5
    description: "devmnt fids Arc'd: dup'd open chan Tclunks exactly once; namec walk intermediates Tclunked on Ok AND Err exits; the 05-09 redirect-clone test unmodified"
    requirement: QUICK-261104-B4P
    verification:
      - kind: unit
        ref: "r9 port/tests/devmnt_test.rs#{devmnt_dup_close_clunks_exactly_once, devmnt_namec_walk_sweeps_intermediate_fids, devmnt_namec_walk_error_clunks_intermediates} + devmnt_mount_redirect_clones_the_table_fid (unmodified)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Harvey's in-tree mk parses nested rc backquotes (donor pin 350266104f depth counter); sh-style control unchanged"
    requirement: QUICK-261104-B4P
    verification:
      - kind: integration
        ref: ".build/bquote-repro/prefix.log (corruption: X=} Y=sh-ok) vs postfix.log (fixed: X=bquote-ok Y=sh-ok) via build/aarch64/bquote-repro.sh"
        status: pass
    human_judgment: false
  - id: D7
    description: "flake.nix systems list of three; nix flake show --all-systems green; aarch64-darwin native green; linux .type legs 'app'; x86_64-darwin eval fails absent"
    requirement: QUICK-261104-B4P
    verification:
      - kind: integration
        ref: "the Task-3 verify block (nix flake show --all-systems && the eval matrix) — ALL GREEN"
        status: pass
    human_judgment: false
  - id: D8
    description: "Both deferred ledgers: eight resolutions + two evidence-only status flips with evidence lines; the input-wedge entry stays open"
    requirement: QUICK-261104-B4P
    verification:
      - kind: other
        ref: "grep — 05 file: 10 'status: resolved' + the single open input-wedge entry; 02 file row 5 'resolved (261104-b4p)'"
        status: pass
    human_judgment: false

duration: 31min
completed: 2026-11-04
status: complete
---

# Quick task 261104-b4p: Resolve Eight Deferred Items Before the v1.0 Tag — Summary

**Cooked console echo, exit-time fd-group sweep, note-interruptible sys_sleep, and Arc'd devmnt fids landed in r9 with grown host suites; Harvey's mk gained the donor's bquote depth counter, the flake dropped x86_64-darwin, and both deferred ledgers show eight resolutions plus two evidence-only flips.**

## Performance

- **Duration:** 31 min
- **Started:** 2026-11-04T13:18:22Z
- **Completed:** 2026-11-04T13:49Z
- **Tasks:** 3
- **Files modified:** 22 (12 r9, 10 harvey)

## Accomplishments

- **Item 1 (echo):** devcons's cooked input drain now echoes typed bytes through a fifth ConsHooks field (`echo: fn(&[u8])`), collected under the INPUT lock into a bounded Vec (ECHO_CAP = 2·INPUT_CAP + 3·LINE_CAP, drop-excess) and emitted after the lock drops; arch-wired to `write_bytes` in main.rs. Raw mode, ^U, ^D, DEL stay silent; BACKSPACE echoes BS-SP-BS only on a real pop; completers echo CRLF.
- **Item 2 (#c/pid):** the 05-09 implementation pinned host-side (`pid_file_serves_the_callers_pid`) and gate-side (both drivers' final `pid-warning-absent` stage — the whole capture must carry no `#p/0/ctl` fragment).
- **Item 4 (exit sweep):** `fdgrp::drain_all` + exit_tail's FIRST-step detach-and-last-sharer sweep; close dispatch never under the group lock; structural exclusions (devsrv registry clone, parked paqfs io child) commented.
- **Item 6 (interruptible sleep):** sys_sleep's pending-note pre-check + local-deadline compare; tick_timeouts reordered alarm-first then note-aware wake. Proven end-to-end by smokesleep's `smokesleep: interrupted` marker asserted at AllExited in all three exec-family profiles — with an e2e RED proof against the pre-fix kernel (marker absent, sleep ran to expiry, `r=0 seen=1`).
- **Item 5 (devmnt fids — LANDED, no escape hatch):** `Chan.mnt_fid: Option<Arc<u32>>`; open shares the walked fid's Arc; `clunk_if_last` (count==1) drives every DEV_MNT close, copen-independent; namec's walk sweeps minted intermediates on every exit (Ok and Err); syscall callers close-dispatch walked-past/resolved chans. Three new FakeServer pins incl. the Tclunk record; the 05-09 redirect-clone regression test passes unmodified.
- **Item 3 (mk bquote):** the donor pin 350266104f's 3-line depth counter in Harvey's lex.c — pre-fix corruption captured (`.build/bquote-repro/prefix.log`: `X=} Y=sh-ok`), post-fix correct (`postfix.log`: `X=bquote-ok Y=sh-ok`).
- **Item 8 (flake):** systems list of three; `nix flake show --all-systems` green; the x86_64-darwin eval fails with the attribute absent.
- **Item 7 + two status flips:** HowItWorks namespace-core re-verified (all five mandated topics present — no patch); the pgrp-lock wedge (WR-04, r9 36dcc39) and 3-string stat records (CR-01, r9 a244ba2) flipped evidence-only. The input-wedge-flake entry stays open (documented-not-fixed).

## Task Commits

r9 main (test → feat per TDD):

1. **Task 1 RED:** `c713ce1` — test(261104-b4p): failing cooked-echo arms + the #c/pid pin
2. **Task 1 GREEN:** `fe8b125` — feat(261104-b4p): cooked-mode console echo in the devcons input drain
3. **Task 2 RED (port):** `628adc1` — test(261104-b4p): failing lifecycle pins — drain_all + the Arc'd fid discipline
4. **Task 2 RED (e2e):** `8538e02` — test(261104-b4p): boot-test asserts the smokesleep interrupt marker
5. **Task 2 GREEN:** `fdeba94` — feat(261104-b4p): lifecycle cluster — exit fd-group sweep + interruptible sys_sleep + Arc'd devmnt fids

harvey master:

6. **Task 1 gates+flips:** `6979ecccb` — fix(261104-b4p): gates cope with console echo + deferred flips (items 1+2)
7. **Task 2 stage inputs:** `00fb40692` — feat(261104-b4p): smokesleep + staging wiring
8. **Task 2 flips:** `2e1d27ff8` — fix(261104-b4p): flip exit-sweep, sleep, and fid-lifecycle deferred entries (items 4+5+6)
9. **Task 3:** `b1d600114` — fix(261104-b4p): mk bquote depth counter + flake systems drop + the ledger flips (items 3+7+8)

## Gate Evidence

| Gate | Result | Evidence |
|------|--------|----------|
| cargo test -p port | 80/80 green (grew from 76: echo arms + pid pin + drain_all + three fid pins) | b4p-task2-green-port2.log |
| boot-test ×5 (virt virtexec virtmulti virtnotes virtpanic) | all PASS; `smokesleep: interrupted` ×1 in each exec-family capture | r9/target/boot-test-*.log |
| virtshell build + clippy | green (34 pre-existing warnings, 0 errors) | b4p-task2-gates.log |
| drive-r9 TWICE consecutive | ALL STAGES PASS ×2 with echo on the wire + pid-warning-absent | build/aarch64/drive-r9.log |
| full smoke (`--override-input r9 git+file:///Users/neilskinner/code/r9`) | ALL STAGES PASS (exit 0) incl. the restart loop | b4p-task2-smoke.log |
| mk repro pre/post | prefix.log corrupts (`X=}`), postfix.log correct | .build/bquote-repro/*.log |
| flake eval matrix | show --all-systems exit 0; aarch64-darwin green; linux .type legs "app"; x86_64-darwin absent | Task-3 verify block |

## Deviations from Plan

**1. [Rule 3 — blocking] The plan's `<verify>` invocations omit `--arch aarch64`** (cargo xtask defaults to x86_64 — the same documented shape as 261103-p64's deviation 5). All xtask gates ran with `--arch aarch64`. No file changes.

**2. [Rule 2 — correctness] Caller-side DEV_MNT close sweeps beyond the plan's named set.** The plan names the namec walk sweep + sys_create's dot-redirect clone. Under open-shares-the-Arc, sys_exec's walked chan (dropped after open_chan) would have stranded every exec'd command's fid — a REGRESSION vs the value-copy era's exactly-once (close(opened) saw count 2 and skipped). Added one-line close-dispatches at sys_exec (both arms + error paths), sys_open's error path, sys_stat, sys_chdir (error + replaced dot), and the sys_bind/sys_mount/sys_unmount target chans — all ride clunk_if_last's count check (always safe, never touches shared/table fids). Documented in the fdeba94 commit body + the item-5 flip.

**3. [Rule 3 — blocking, environment] drive-r9 needed `fdtput` on PATH** — the harvey devshell ships qemu/expect but no dtc (run-r9.sh's own header notes dtc comes from the r9 side). The gate invocations prepended the nix-store dtc bin dir to PATH. No file changes.

**4. [Rule 3 — blocking, host tooling] The pristine/working host-mk evidence builds needed plan9port-era compat fixups** in the SCRATCH copies only (never the working tree): the `waitfor`→`mkwaitfor` rename (plan9port libc.h's `#define waitfor p9waitfor` collides with mk's own), the regexp9 union-member Resub (`match->sp`→`match->s.sp`), `-std=gnu89` (clang rejects Plan 9 C's implicit-int), the `/bin/rc` shell path → `$PLAN9/bin/rc`, and `-lregexp9` at link. build/aarch64/bquote-repro.sh documents each inline; lex.c under test stayed pristine.

**Test-fixture correction (in GREEN):** devmnt_test's dup pin gained `drop(opened)` after its close dispatch — the kernel pops the chan out of its fd slot and the value drops right after dispatch, so the next close must see the true remaining Arc count; the RED commit's failure point (the unconditional-clunk assertion) is unchanged by the correction.

**Total deviations:** 4 + 1 fixture correction — all necessary for correctness/environment; no scope creep.

## Issues Encountered

- The e2e RED proof for sys_sleep needed the pre-fix kernel against the NEW xtask assertion: done via a patch-dance (working-tree diff of aarch64/src/{syscall,proc}.rs saved, HEAD restored, `boot-test --config virtexec` failed on exactly the absent marker, GREEN re-applied). Recorded in 8538e02's commit body.
- Item 5's honesty escape hatch was never needed: the Arc'd-fid rework held every gate on the first landing.

## Item-5 Outcome

**LANDED.** No attempted-reverted-reason record — the named shape (Arc'd fid, clunk_if_last, copen-independent close arm, namec walk sweep, sys_create sweep) held all gates green on landing.

## Deferred-Ledger Flip List

05 file (8): echo → resolved; #c/pid → resolved; exit-sweep → resolved; fid lifecycle → resolved (LANDED); sleep → resolved; HowItWorks namespace-core → resolved (re-verified, cbbc8e4); pgrp-lock wedge → resolved (STATUS FLIP, WR-04 r9 36dcc39); 3-string stat records → resolved (STATUS FLIP, CR-01 r9 a244ba2); x86_64-darwin → resolved. **Stays open:** the input-wedge-flake entry (not selected — documented-not-fixed).

02 file (1): row 5 (mk bquote) → resolved (261104-b4p) with the repro evidence; the BOOTFSPROTOS shim note preserved.

## Self-Check: PASSED

- All nine commits verified present (r9: c713ce1 fe8b125 628adc1 8538e02 fdeba94; harvey: 6979ecccb 00fb40692 2e1d27ff8 b1d600114).
- All key files exist on disk (12 r9 modified, 8 harvey modified, 2 harvey created: smokesleep.c, bquote-repro.sh).
- Evidence files exist: build/aarch64/drive-r9.log, .build/bquote-repro/{prefix,postfix}.log, r9/target/boot-test-*.log.
- Ledgers: 05 file 10× `status: resolved` + the input-wedge `status: open` (correct); 02 file row 5 `resolved (261104-b4p)`.

## Next Phase Readiness

- The deferred ledger is clean for the v1.0 milestone tag: zero stale open entries that are actually fixed, no known-cheap fixes left behind.
- The r9 pin advisory stands for milestone close: harvey's flake.lock r9 pin (9549cee) is now further behind r9 main (b4p series on top of the review-fix/unsafe-elimination waves) — bump at milestone completion with a fresh hosted run; the pinned-default smoke is the milestone-tag gate, not this task.
- The input-wedge-flake entry is the top debug candidate for the CI leg (never reproduced under the cons watchdog).

---
*Quick task: 261104-b4p*
*Completed: 2026-11-04*
