---
phase: 05-shell-boot-one-command-integration
plan: 08
subsystem: kernel
tags: [r9, aarch64, devcons, cooked-mode, consctl, raw-mode, DEL, note-groups, notepg, RFNOTEG, sleep, alarm, generic-timer, scheduler, pick_next, expect, qemu, interactive-gate]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-07)
    provides: the boot-to-prompt hinge (boot completes to the literal `% ` and rc hits the designed cons-read wall); the note machinery + the 05-01 race fix (the DEL path's delivery substrate); devmnt/the mounted tree (rc execs through it); the tick + the 04-07 block/wake machinery
  - phase: 05-shell-boot-one-command-integration (05-01)
    provides: the trap_return note-race fix the gate's DEL stage re-proves under the real chain
  - phase: 05-shell-boot-one-command-integration (05-03)
    provides: the real-chain trace that gated SLEEP(17) (×42) and ALARM(6) (×1)
provides:
  - the devcons READ path (port/devcons.rs): the bounded 4 KiB input queue (drop-oldest + one-line warning, T-05-15), the cooked line discipline (0x08 erase / 0x15 kill-line / CR→LF / newline-inclusive complete lines / ^D ends-unbuffered), blocking PREAD, DEL (0x7f) interception (flush partial + post "interrupt" to the readers' note groups + fail the read -1/"interrupted"), raw mode, and the /dev/consctl file (rawon/rawoff + mode-word readback)
  - note GROUPS (the donor's noteid): Proc.noteid, fresh-per-construction, rfork inheritance (RFPROC: inherit unless RFNOTEG; non-RFPROC RFNOTEG re-groups the caller), and post_note_group_of_members (the pgrpnote port — the plan's sanctioned scoping for the notepg machinery)
  - the cons-read wake: the tick readies cons-waiting Blocked procs when the PL011 RX ring is non-empty (the lock-free peek + the reschedule()-precedent PROC_TABLE take from trap context)
  - sys_sleep + sys_alarm (donor syssleep/tsleep + procalarm/alarmkproc semantics on the generic timer; init's restart-loop sleep(1000) is real now)
  - the pick_next scheduler fix: the current proc is no longer skipped (a readied current is resumable — the cons-read deadlock's fix)
  - build/aarch64/drive-r9.expect — the INTERACTIVE GATE (05-12's smoke core): staged boot assertions + echo round-trip + DEL safety + exit→restart
  - evidence: .planning/phases/05-shell-boot-one-command-integration/05-08-drive-r9.log (the all-pass run at r9 6645e29)
affects: [05-09 (the smoke sweep + devproc ride the interactive prompt), 05-11/05-12 (the composition app + smoke grow from run-r9.sh + drive-r9.expect — the launcher's stdin fix is load-bearing), rio/devdraw era (consctl raw mode is its prerequisite)]

# Actuals (#2632) — pairs with the plan's estimate (100000 tokens, confidence: low).
actuals:
  tokens: 21869    # MEASURED chars/4: r9 code diff 51530 + r9 docs 8016 + harvey 6929 + deferred-items delta 2353 + this SUMMARY ~18600 = 87478/4 ≈ 21869; the estimate (100k) overshot ~4.6x
  tasks: 3
  commits: 5       # MEASURED: r9 git rev-list --count 454f0f2..8ce9a92 = 4; harvey 569783d3..a4d7162ae = 1 (pre-metadata; the docs commit rides on top, per the 05-04..07 convention)
plan_head_before: {r9: "454f0f27190f720f828d7ddf64c99a80cf009152", harvey: "569783d3376480157af5e3242a6a782e48f8d39b"}
plan_head_after: {r9: "8ce9a92 (docs; code at 6645e29)", harvey: "a4d7162ae (pre-metadata; the docs commit rides on top)"}

# Tech tracking
tech-stack:
  added: []        # zero new dependencies anywhere (T-05-SC held: no package installs)
  patterns:
    - "the cons input engine pattern: the IRQ handler pushes a lock-free ring; the ring→queue drain + cooked/raw processing happen ONLY in the reader's syscall context; a blocked reader is readied by the tick's lock-free-pending peek (IRQ context never takes the cons lock — the extended locking rule)"
    - "note groups (the donor's noteid): fresh id per construction, rfork inheritance, non-RFPROC RFNOTEG re-groups the caller; the console DEL posts to the READERS' groups (the foreground-group scoping for the notepg machinery r9 lacks)"
    - "the tick as the timeout/wake sweeper (tick_timeouts: sleep expiry + alarm note posts; cons_input_wake): PROC_TABLE from tick (trap) context per the reschedule() precedent"
    - "POSIX background-stdin trap: a non-interactive sh redirects a backgrounded command's stdin from /dev/null — a launcher spawning qemu's stdio serial in the background MUST pass <&0 explicitly (run-r9.sh)"
    - "expect 5.45 (darwin /usr/bin/expect): log_user 0 silently disables log_file writes — the capture stays empty; keep log_user on"

key-files:
  created:
    - ../r9/port/tests/devcons_test.rs
    - build/aarch64/drive-r9.expect
    - .planning/phases/05-shell-boot-one-command-integration/05-08-drive-r9.log
  modified:
    - ../r9/port/src/devcons.rs
    - ../r9/aarch64/src/syscall.rs
    - ../r9/aarch64/src/proc.rs
    - ../r9/aarch64/src/main.rs
    - ../r9/aarch64/src/trap.rs
    - ../r9/aarch64/src/platform/virt/pl011.rs
    - ../r9/aarch64/src/platform/virt/timer.rs
    - ../r9/HowItWorks.md
    - build/aarch64/run-r9.sh
    - .gitignore
    - .planning/phases/05-shell-boot-one-command-integration/deferred-items.md

key-decisions:
  - "the DEL target scoping (the plan's sanctioned delta): r9 grows real note-group ids (Proc.noteid + RFNOTEG semantics per 9k sysproc.c), and the console DEL posts 'interrupt' to the note GROUPS of the console's readers (blocked waiters ∪ the current reader) — the foreground group this era; init's rfork(RFNOTEG)'d shell children are interrupted WITHOUT init itself getting the note (the donor's pgrpnote shape)"
  - "DEL discards the in-assembly partial line (the gate's shape: the fresh command after a DEL must not inherit the partial) and fails the reading syscall -1/'interrupted' (the donor's qread-under-a-note → rc's Eintr path re-prompts)"
  - "no input ECHO (documented devcons delta): typed bytes never re-write to the wire — D-06/the plan never name it, and its absence keeps the gate's output markers unambiguous (a human at the prompt sees no typed echo; deferred-items entry)"
  - "the cons wake rides the tick (1 ms) — the IRQ handler stays lock-free (the plan's extended locking rule); a blocked reader's register-then-block can't lose the wake (syscalls serialized, IRQ-masked)"
  - "sleep(0) yields via preempt() (the donor's yield); a sleeping proc wakes only on expiry this era (the donor's note-interruptible tsleep is a documented era limit — no measured caller interrupts a sleep)"
  - "alarm(0) disarms and returns the previously-remaining ms (the donor procalarm); init's traced alarm call is exactly that disarm"

patterns-established:
  - "expect-driver stage pattern for the r9 shell gates: staged ordered assertions (banner → paqfs... → stacksmoke → `% `), stage-named timeout failures, panic-marker fast-fail via expect_before, EXIT-path teardown (process-group kill + bracket-fixed pkill), fresh capture per run (D-16), TCG-sized timeouts — 05-12's smoke grows from drive-r9.expect"
  - "the scripted-feed host-test idiom for the blocking console read (devcons_test.rs mirrors devpipe_test.rs: the block hook plays the host typing while 'we' are parked)"

requirements-completed: [INT-01]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "the devcons read path: cooked line discipline (erase 0x08 / kill-line 0x15 / CR→LF / newline-inclusive lines / ^D ends-unbuffered), blocking PREAD, raw mode + /dev/consctl switching with mode-word readback, the 4 KiB drop-oldest bound (T-05-15)"
    requirement: INT-01
    verification:
      - kind: unit
        ref: "cargo test -p port — port/tests/devcons_test.rs (cons_read_path_cooked_raw_del_and_consctl): the split-feed block count, erase/kill editing, ^D, DEL interruption + partial flush + the surviving fresh line, consctl switching + readback, raw passthrough (DEL/0x08 as data), the 5000-byte overrun cap — plus the full suite 61/61"
        status: pass
      - kind: e2e
        ref: "05-08-drive-r9.log: the gate's echo round-trip (`echo r9shell7` → `r9shell7` → fresh `% `) — typed input read, cooked, answered from EL0"
        status: pass
    human_judgment: false
  - id: D2
    description: "DEL (0x7f) → 'interrupt' note to the console readers' note groups (the notepg-scoping delta, real noteid machinery + RFNOTEG semantics); DEL never enters the line buffer; rc is interrupted, not killed (re-prompts and answers again); the 05-01 race fix re-proven under the real note chain"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "05-08-drive-r9.log's DEL stage: partial `fnord` + DEL → rc's re-prompt (`% `) → fresh `echo r9delok` answers `r9delok` (the partial did NOT corrupt the fresh command); zero 'has no notify handler' kill lines (rc's notifyf caught the note, Eintr re-prompted)"
        status: pass
      - kind: unit
        ref: "devcons_test.rs DEL arm: read errs 'interrupted', the post records ('interrupt', [TEST_PID]), the partial is flushed"
        status: pass
    human_judgment: false
  - id: D3
    description: "sys_sleep + sys_alarm (trace-gated: sleep ×42, alarm ×1 in the 05-03 trace) riding the generic timer — donor syssleep/tsleep + procalarm/alarmkproc semantics; init's for(;;) restart loop sleeps 1s for real"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "05-08-drive-r9.log's exit→restart stage: `exit` → `init: rc exit status` → `init: starting /bin/rc` (after the real sleep(1000)) → fresh `% `; ZERO `syscall 17 unimplemented` and ZERO `syscall 6 unimplemented` lines (05-07's designed gaps are gone)"
        status: pass
      - kind: integration
        ref: "the staged-profile fan (virtexec/virtmulti/virtnotes/virt/virtpanic) all PASS on the final tree — sleep/alarm inert for binaries that never call them"
        status: pass
    human_judgment: false
  - id: D4
    description: "the interactive gate driver itself (build/aarch64/drive-r9.expect, committed — 05-12's smoke core): staged assertions, TCG-sized timeouts, CRLF-tolerant presence-only markers, panic fast-fail, fresh capture per run, provable qemu teardown"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "the gate run: ALL STAGES PASS, exit 0; the teardown leg (`pgrep -f qemu-system-aarch64` after the run) finds no orphan — the EXIT-trap process-group kill worked"
        status: pass
    human_judgment: false
  - id: D5
    description: "the gate-caught fixes (the iterate-discipline's yield): run-r9.sh's background-stdin starvation (the POSIX `&` rule → the explicit <&0), and the pick_next skip-current scheduler deadlock (a tick-readied current proc was unpickable)"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "pre-fix the guest's PL011 RX FIFO stayed empty no matter what was typed (measured via a qemu register dump + chardev logfile: qemu never read the channel) and the fixed launcher's guest answers; pre-fix the wake readied the waiter but pick_next never resumed it (the wake-scan dump), post-fix the full gate passes"
        status: pass
      - kind: integration
        ref: "the full fan + host tests re-green on the final tree (the scheduler fix touches the shared pick path)"
        status: pass
    human_judgment: false

# Metrics
duration: 223 min (incl. the qemu input-delivery investigation + two gate debug cycles + the fan)
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 08: devcons read path + SLEEP/ALARM + the interactive gate Summary

**ROADMAP SC#1 is functionally true: the `% ` prompt is a conversation. devcons learned to READ — the cooked line discipline (erase/kill-line/CRLF-tolerant), consctl raw switching, and DEL (0x7f) posting "interrupt" to the console readers' note groups via the new real note-group machinery (Proc.noteid + RFNOTEG semantics, the plan's sanctioned scoping for notepg) — plus sys_sleep/sys_alarm riding the generic timer (init's restart loop sleeps for real; both were measured in the 05-03 trace). The committed expect gate proves it end-to-end: staged boot → the literal prompt → `echo r9shell7` answers → DEL mid-command interrupts without killing rc → `exit` restarts rc through init's for(;;) loop to a fresh prompt. The gate's iterate-discipline caught two real bugs on the way: run-r9.sh's backgrounded qemu spawn silently starved console input (the POSIX non-interactive-sh stdin→/dev/null rule; the explicit `<&0` is load-bearing now), and pick_next's skip-current rule deadlocked any proc readied while still current (the tick wakes with current == the waiter).**

## Performance

- **Duration:** 223 min
- **Started:** 2026-11-03T10:25:58Z
- **Completed:** 2026-11-03T14:09:04Z
- **Tasks:** 3
- **Files modified:** 11 (+1 r9 test created, +1 harvey driver created, +1 harvey evidence log created)

## Accomplishments

- **Task 1: the console read path** (r9 `10cb2a6`): port/devcons.rs grew the input engine — the bounded 4 KiB input queue (drop-oldest + one-line console warning on overrun, T-05-15; the IRQ handler never blocks and never grows unbounded), the cooked discipline ported from 9k's devcons.c (line buffering; 0x08 erase; 0x15 kill-line; `\r`→`\n` accepted as Enter; completed lines include the trailing newline; ^D ends the line unbuffered), blocking PREAD (blocks until a complete line exists, riding the 04-07 block/wake machinery via the injected ConsHooks), raw mode, and the `/dev/consctl` file (rawon/rawoff + mode-word readback — the plan's extension of the donor's write-only file; rawon completes a pending partial line per the donor's ch=0 marker). DEL (0x7f) is never buffered: it flushes the partial line, posts "interrupt" to the readers' note groups, and fails the read -1/"interrupted" (the donor's qread-under-a-note → rc's Eintr). The arch side grew: Proc.noteid (the donor's noteid — fresh per construction; rfork inherits unless RFNOTEG; non-RFPROC RFNOTEG re-groups the caller, so init's shell children take DELs without init ever seeing one), the cons block/wake hooks (the tick readies cons waiters when the RX ring is non-empty — the reschedule() lock-from-trap-context precedent), and post_note_group_of_members (the pgrpnote port). The wake rides the tick; the drain rides the reader's syscall context — IRQ context never takes the cons lock (the plan's extended locking rule, documented in syscall.rs's header + devcons.rs's module header). Host tests (port/tests/devcons_test.rs, the devpipe scripted-hook idiom) pin cooked/raw/DEL/consctl/overrun/blocking; cargo test -p port 61/61 green.
- **Task 2: sys_sleep + sys_alarm** (r9 `8c29165`): both trace-gated (sleep ×42 — init's restart-loop sleep(1000); alarm ×1 — init's alarm(0) disarm). Donor semantics: sleep(0) yields (preempt()), sleep(ms) parks until the tick passes now+ms (1 tick = 1 ms; register-then-block can't lose the wake); alarm arms/resets/disarms and returns the previously-remaining ms; expiry posts the "alarm" note from the tick's tick_timeouts sweep (the donor alarmkproc's postnote arm; delivery rides the 04-08 boundary). Documented era limit: the donor's tsleep is note-interruptible; r9's sleeper wakes on expiry only (no measured caller interrupts a sleep). The staged fan proved them inert where unused.
- **Task 3: the INTERACTIVE GATE** (harvey `a4d7162ae`): build/aarch64/drive-r9.expect (committed, not a heredoc — 05-12's smoke grows from it) spawns run-r9.sh interactive and asserts, in order: banner → `paqfs...` → `stacksmoke: depth 64 ok` → the literal `% ` → `echo r9shell7` answers `r9shell7` → fresh `% ` → partial "fnord" + DEL → rc re-prompts (interrupted, alive) → `echo r9delok` answers (the partial didn't corrupt it) → `% ` → `exit` → `init: starting /bin/rc` → fresh `% ` (the D-01 restart loop evidenced end-to-end). ALL STAGES PASS, exit 0; provably no orphaned qemu after the run. The capture is committed as evidence (05-08-drive-r9.log) — and the log shows ZERO `syscall 17`/`syscall 6` unimplemented lines (05-07's designed gaps are gone) and zero panic lines.
- **The gate-caught fixes** (the iterate-discipline's yield — two real bugs): (1) **run-r9.sh never delivered console input** (r9-side visible as the PL011 RX FIFO staying empty no matter what was typed): a non-interactive sh redirects a backgrounded command's stdin from /dev/null (POSIX), and the launcher's `"$@" &` starved qemu's stdio serial — the explicit `<&0` redirect is load-bearing now (harvey `a4d7162ae`). The debug trail: the echo-profile kernel echoed over identical transport/argv while the virtshell kernel didn't, which read as a kernel bug until a qemu register dump (FR empty, IMSC/CR armed) + a chardev `logfile=` capture + a `-trace pl011_*` run proved qemu never read the channel — then the minimal `cat &` repro isolated the shell rule. (2) **pick_next skipped the current proc** (r9 `6645e29`): a proc that blocks via sched() REMAINS current while the sched loop runs, so when the tick's cons_input_wake readied it (Blocked→Ready), the scan skipped right past it — the waiter sat Ready-but-unpickable forever (the wake-scan dump showed it). The pipe/await wakes never tripped this because their waker runs in a DIFFERENT proc's context; the tick wakes with current == the waiter. The skip is removed (the scan still starts at cur+1 — the readied current is picked last, fairness kept); the full fan re-green.

## Task Commits

Each task was committed atomically (two-repo convention — r9 commits land on `main` of `requaos/r9`; harvey commits on master):

1. **Task 1: devcons read path — RX ring → bounded queue → blocking PREAD + cooked discipline + consctl + DEL→interrupt note groups** — r9 `10cb2a6` (feat)
2. **Task 2: sys_sleep + sys_alarm riding the generic timer (trace-gated)** — r9 `8c29165` (feat)
3. **Task 3 gate fix: pick_next no longer skips the current proc** — r9 `6645e29` (fix)
4. **Task 3: drive-r9.expect + the run-r9.sh stdin fix + the evidence log + .gitignore** — harvey `a4d7162ae` (feat)
5. **HowItWorks.md material-change section (the header's mandate)** — r9 `8ce9a92` (docs)

**Plan metadata:** (the harvey docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS + deferred-items)

## Files Created/Modified

- `../r9/port/src/devcons.rs` — the read path: the ConsInput engine (bounded queue, cooked/raw, consctl, DEL), the ConsHooks injection, the module header's full semantics + locking + scoping documentation
- `../r9/port/tests/devcons_test.rs` — the scripted-hook host tests (cooked/raw/DEL/consctl/overrun/blocking)
- `../r9/aarch64/src/proc.rs` — Proc.noteid + cons_waiting + sleep_until/alarm_at fields; cons_block_current/cons_input_wake/post_note_group_of_members/rfork_noteg_split/tick_timeouts; the pick_next fix
- `../r9/aarch64/src/syscall.rs` — SLEEP(17)/ALARM(6) dispatch + handlers; the RFNOTEG pub(crate) + non-RFPROC arm; the header's extended locking rule
- `../r9/aarch64/src/main.rs` — the cons hooks registration (dev_exec + virt)
- `../r9/aarch64/src/platform/virt/pl011.rs` — cons_rx_pending (the lock-free peek)
- `../r9/aarch64/src/platform/virt/timer.rs` — ticks() accessor + the tick's tick_timeouts/cons_input_wake calls
- `../r9/HowItWorks.md` — the 05-08 section (mandated)
- `build/aarch64/drive-r9.expect` — the interactive gate driver (NEW, committed)
- `build/aarch64/run-r9.sh` — the `<&0` stdin fix (Rule 3)
- `.gitignore` — the transient gate log entry
- `.planning/phases/05-shell-boot-one-command-integration/05-08-drive-r9.log` — the committed gate evidence
- `.planning/phases/05-shell-boot-one-command-integration/deferred-items.md` — two era-limit entries

## Decisions Made

- **The DEL target scoping** (the plan's sanctioned delta, recorded in devcons.rs's module header): r9 grows real note-group ids (Proc.noteid, rfork inheritance per 9k sysproc.c:183, non-RFPROC RFNOTEG per :82) and the console DEL posts "interrupt" to the note GROUPS of the console's readers (blocked waiters ∪ the current reader) — the foreground group this era. init never gets the shell's DELs.
- **DEL discards the in-assembly partial line** and fails the read -1/"interrupted" — the gate's contract (a fresh command after DEL must not inherit the partial) and the donor's qread-under-a-note shape (rc's Eintr re-prompts).
- **No input echo** (documented delta — deferred-items entry): nothing in D-06/the plan names it, and its absence keeps the gate's output markers unambiguous; a human at the prompt sees no typed echo this era.
- **The cons wake rides the tick** (1 ms latency, invisible to typing speeds) — the IRQ handler stays lock-free per the plan's extended locking rule.
- **sleep(0) yields via preempt()** (the donor's yield); sleepers wake only on expiry this era (documented era limit).
- **alarm is implemented** (the trace measured it once — init's disarm): arm/reset/disarm + the previously-remaining-ms return + expiry posting "alarm" per the donor's procalarm/alarmkproc.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking, gate-surfaced] run-r9.sh's backgrounded qemu spawn starved console INPUT**
- **Found during:** Task 3's first gate run — the boot stages passed but the echo stage never answered; the guest showed zero RX activity
- **Issue:** the launcher spawns qemu with `"$@" &`; a NON-INTERACTIVE sh (no job control) redirects a backgrounded command's stdin from /dev/null (POSIX), so typed bytes never reached qemu — the guest's PL011 RX FIFO stayed empty with the guest armed and idle (measured via a temporary register dump: FR=RXFE, IMSC/CR armed; a chardev `logfile=` capture showed qemu never read the channel; a `-trace pl011_*` run showed can_receive polled but no receive; the minimal repro `echo x | sh -c 'cat & wait'` returned empty vs `<&0` returning x)
- **Fix:** `"$@" <&0 &` (both the setsid and the plain legs) + the header/inline comment naming the rule
- **Files modified:** `build/aarch64/run-r9.sh`
- **Verification:** the echo-profile kernel echoes typed input through the fixed launcher; the full gate passes
- **Committed in:** `a4d7162ae` (harvey)

**2. [Rule 1 - Bug, gate-surfaced] pick_next skipped the CURRENT proc — a tick-readied waiter deadlocked**
- **Found during:** Task 3's debug — the wake path was instrumented (the tick saw the pending ring every tick; cons_input_wake printed `readied 1`; the waiter sat Ready forever, never resumed)
- **Issue:** a proc that blocks via sched() REMAINS the current proc while the sched loop runs; pick_next's `if Some(i) == cur_idx { continue; }` skipped it even after the tick readied it — the console read deadlocked Ready-but-unpickable. The pipe/await wakes never tripped this because their waker runs in a DIFFERENT proc's context (the wakee is not current then); the tick wakes with current == the waiter
- **Fix:** the skip removed — the round-robin scan still STARTS at cur+1 (the readied current is picked last, fairness kept); at pick_next entry the current proc is normally Blocked/Moribund (state-excluded), so the only behavior change is that a readied current is resumable
- **Files modified:** `../r9/aarch64/src/proc.rs`
- **Verification:** the gate passes end-to-end; the full boot-test fan + host tests re-green on the final tree (the fix touches the shared pick path)
- **Committed in:** `6645e29` (r9)

**3. [Process note] the TEMP debug instrumentation came and went uncommitted**
- The bisect used lock-free raw-DR markers (a `U` per INTID-33 dispatch, a `.` per drained byte, a per-1000-tick UART register dump, wake-scan table dumps) — all removed before any commit; the committed tree carries none of it. One build failure mid-debug (a u8/u32 write_reg type mismatch) was fixed in-flight.

---

**Total deviations:** 2 auto-fixed (1 Rule 3 blocking — the launcher stdin starvation; 1 Rule 1 bug — the scheduler deadlock) + 1 process note
**Impact on plan:** Both auto-fixes are the gate's iterate-discipline working as designed (run the gate, let the evidence name the gap). No scope creep: the scheduler fix is the minimal semantics restoration (the donor's scheduler never skips a readied proc); the launcher fix is one redirect.

## Issues Encountered

- **The qemu input-delivery investigation dominated the duration.** The failure read as a kernel bug (the echo-profile kernel received input over identical transport/argv; the virtshell kernel didn't) and took a multi-hypothesis bisect (pty vs pipe vs socket transports; loader/patched-DTB exonerated; GIC/UART guest state verified armed; a register dump proving the RX FIFO empty; a chardev logfile proving qemu never read; a qemu trace proving the watcher polled but no receive fired — then the minimal shell repro isolated the POSIX background-stdin rule). AFTER that fix, a second layer surfaced (the pick_next deadlock) via the wake-scan dump. Both fixed and re-proven by the gate.
- **expect 5.45 (darwin /usr/bin/expect) quirk:** `log_user 0` silently disables `log_file` writes (the capture file is created empty — measured). The driver keeps log_user on (the wire echo on the driver's stdout is harmless); documented in the driver's header.
- **probe artifacts:** several early probes booted a stale frozen kernel copy (a probe's hardcoded path) — a red herring mid-debug; the final evidence (05-08-drive-r9.log) is from the committed tree (r9 code at `6645e29`, docs at `8ce9a92`).

## User Setup Required

None - no external service configuration required. (Driving the gate by hand needs qemu-system-aarch64 + fdtput + expect on PATH — the r9 devshell provides qemu/dtc; expect is the bare host's /usr/bin/expect; the exact invocation is the plan's verify command.)

## Known Stubs

- **A proc blocked in sys_sleep wakes only on expiry** (documented era limit; the donor's tsleep is note-interruptible) — deferred-items.md entry; no measured caller interrupts a sleep this era.
- **No console input echo** (documented devcons delta) — typed bytes never re-write to the wire; deferred-items.md entry (the rio/kbdfs era owns echo).
- **The notepg machinery is NOT grown** — the plan's sanctioned scoping (the readers' note groups) stands in; documented in devcons.rs's module header.
- 05-04..05-07's known stubs are unchanged except as resolved here: the cons read wall is GONE (this plan demolished it), SLEEP/ALARM are live, FAUTH stays loud-None (the designed D-02 path), devproc ctl + #c/pid remain 05-09's, the pgrp-lock-across-blocking-walk hazard stays deferred (unchanged by this plan — the gate's chain is strictly sequential), the proc-table Moribund-slot-pinning era limit persists (the gate's 2-generation run is well under it).

## Threat Flags

None beyond the plan's register. (T-05-15 held: the 4 KiB input queue is bounded with drop-oldest + a one-line warning, host-tested with a 5000-byte flood; the IRQ handler never blocks. T-05-16 held: the DEL post targets only the console readers' note groups — no broadcast; delivery rides the unchanged 04-08 boundary machinery, re-proven by the gate's DEL assertion under the 05-01-fixed race. T-05-SC held: zero package installs.)

## Next Phase Readiness

- **05-09 (devproc + the smoke sweep):** the prompt is interactive — the sweep's `echo`/`ls`/`ps`/`cat` assertions ride the working read path + devmnt reads through the /bin union. `#c/pid` is the named wiring gap for init's pri write (deferred item). drive-r9.expect's stage discipline is the sweep's chassis.
- **05-10/05-11/05-12 (composition + smoke + CI):** run-r9.sh's interactive mode genuinely works now (the `<&0` fix is load-bearing for any stdio-serial launcher); the composition app consumes the proven kernel+image+qemu argv; the smoke grows from drive-r9.expect verbatim.
- r9 HEAD: `8ce9a92` (main; code at `6645e29`). Harvey HEAD: `a4d7162ae` + the metadata commit.
- The staged-profile fan + host tests + clippy are green on the final tree (through the scheduler fix).

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: r9 port/tests/devcons_test.rs (new), port/src/devcons.rs, aarch64/src/{syscall,proc,main,trap}.rs, aarch64/src/platform/virt/{pl011,timer}.rs, HowItWorks.md; harvey build/aarch64/drive-r9.expect (new), build/aarch64/run-r9.sh, .gitignore, .planning/phases/05-shell-boot-one-command-integration/{05-08-SUMMARY.md,05-08-drive-r9.log,deferred-items.md} — all FOUND on disk.
- Commits: r9 10cb2a6 (Task 1), 8c29165 (Task 2), 6645e29 (scheduler fix), 8ce9a92 (docs); harvey a4d7162ae (Task 3 + evidence) — all FOUND in the respective git logs.
- Plan-level <verification> re-run on the final tree: (1) read path — cargo test -p port 61/61 green (incl. the new devcons suite); builds virtshell/virtexec/default/raspi4b + clippy virtexec green; the consctl/rawon/sys_sleep/sys_alarm greps pass; boot-test virt PASS (r9echo7 — the Phase-3 echo gate unregressed); (2) sleep/alarm — the full fan (virtexec/virtmulti/virtnotes/virt/virtpanic) all PASS (inert where unused); (3) the interactive gate — ALL STAGES PASS (banner/paqfs/stacksmoke/prompt/echo-response/echo-prompt/del-reprompt/del-answer/del-prompt/restart-print/restart-prompt), exit 0; `pgrep -f qemu-system-aarch64` after the run is empty (teardown clean).
- Acceptance criteria re-run at final state: Task 1 (cooked reads block until a line completes — gate's echo + the host split-feed test; erase/kill-line edit — host-tested; DEL intercepted and never buffered — host-tested + the gate's DEL stage; raw mode passes bytes unprocessed + consctl switches — host-tested; the IRQ-context lock rule honored — the drain is syscall-context only and the IRQ handler is lock-free; the Phase-3 echo gate + all builds green) — PASS. Task 2 (sleep + alarm implemented with donor semantics on the generic timer — the gate's restart loop rides sleep(1000) and the log has zero syscall-17/6 walls; the fan green) — PASS. Task 3 (the driver asserts the staged boot + the echo round-trip + DEL-without-death + exit→restart, all green on this darwin host; TCG-sized timeouts; CRLF-tolerant presence markers; qemu provably dead after the run; the driver file is committed) — PASS.
