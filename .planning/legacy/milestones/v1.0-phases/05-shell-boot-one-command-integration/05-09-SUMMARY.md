---
phase: 05-shell-boot-one-command-integration
plan: 09
subsystem: kernel
tags: [r9, aarch64, devproc, ps, proc-status, proc-args, ctl, pri, devcons-pid, getpid, namec-dot-accept, devmnt, fid-clone, stat-muid, dirreadall, statcheck, measured-set-closure, expect, qemu, tdd, sweep-gate]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-08)
    provides: the interactive gate driver this plan extends (drive-r9.expect's stage discipline), the cons read path + DEL machinery the sweep rides, the note groups
  - phase: 05-shell-boot-one-command-integration (05-07)
    provides: devmnt + the mount table + the boot-to-prompt hinge (rc execs through the /bin union); the fid-lifecycle divergence note this plan's aliasing fix sharpens
  - phase: 05-shell-boot-one-command-integration (05-03)
    provides: the real-chain trace (05-03-trace.txt) — the measured set this plan closes the dispatch table against
provides:
  - devproc beyond the note slice: /proc root dir read (one stat(5) entry per live pid), pid-dir listing (args/ctl/note/status), status (12 whitespace fields in devproc.c's order — ps.c's tokenizer contract), args (the exec-recorded argv), ctl ("pri N" parsed + recorded) — the 04-08 note path byte-identical
  - Proc.argv (exec-recorded, NUL-joined, ARGV_CAP 256, documented truncation; install_image fills it at EVERY exec) + Proc.pri (rfork-inherited, exec-preserved)
  - #c/pid (devcons Qpid via an injected current-pid hook) — libc getpid() resolves, so init's pri-10 write names the real pid and its designed-degradation warning is GONE from the boot log
  - namec's "." accept (the devwalk Accept arm after the mount consultation) + the fid-clone redirect fix (devmnt::clone_chan — the mount table's server-side fid is never opened/clunked by a redirect)
  - the dispatch-table closure: DISPATCH == the 05-03 measured set ∪ the D-14 named core (REMOVE/WSTAT/FWSTAT/OSEEK never measured → loud-None; FAUTH loud-None by D-02 design)
  - the full D-09 sweep gate (drive-r9.expect): echo + ls /bin (5 subset names) + ps (≥3 pid lines incl rc) + cat /lib/namespace (known line) + DEL + exit→restart — green on darwin with a retained capture
  - evidence: .planning/phases/05-shell-boot-one-command-integration/05-09-drive-r9.log (the all-pass run at r9 1e9323f, docs 69f4843)
affects: [05-10/05-11 (composition consumes the proven kernel+image), 05-12 (the packaged smoke grows from this driver verbatim), 05-13 (CI — the input-wedge flake note is the top debug candidate), the muid hygiene note (devcons/devroot/devenv/devsrv/devdup listings)]

# Actuals (#2632) — pairs with the plan's estimate (70000 tokens, confidence: low).
actuals:
  tokens: 24800    # MEASURED chars/4: r9 code+tests 62588 + r9 docs 6584 + harvey code 5912 + deferred-items delta 3503 + WINDOWS delta 6644 + this SUMMARY ~14000 = 99231/4 ≈ 24800; the evidence log (112 KB capture) excluded per the 05-08 convention; the estimate (70k) overshot ~2.8x
  tasks: 2
  commits: 5       # MEASURED: r9 git rev-list --count 8ce9a92..69f4843 = 4; harvey 4918e34..ca6fd79d3 = 1 (pre-metadata; the docs commit rides on top, per the 05-04..08 convention)
plan_head_before: {r9: "8ce9a92da453b466c261fde0b80a2509e71bd7c2", harvey: "4918e346146a84cd2d0e5f85365d792163c188fc"}
plan_head_after: {r9: "69f4843 (docs; code at 1e9323f)", harvey: "ca6fd79d3 (pre-metadata; the docs commit rides on top)"}

# Tech tracking
tech-stack:
  added: []        # zero new dependencies anywhere (T-05-SC held: no package installs)
  patterns:
    - "the injected proc-table hook set (devproc's ProcHooks: list/info/set_pri) — the set_post_hook dependency-inversion idiom extended from one hook to a hook STRUCT; the arch registers it once at boot under dev_exec (not virt-gated: the /proc + #c surface is platform-independent)"
    - "the 9P fid-clone redirect (devmnt::clone_chan): a mount-table entry's chan is SHARED state — a redirect that will be OPENED must clone the fid (the zero-name Twalk idiom, the donor mntwalk nwname==0 arm), never bare-dup it, or the open/close lifecycle Tclunks the table's fid out from under every later walk"
    - "the stat(5) wire record carries FOUR strings (name/uid/gid/muid) — libc's dirreadall runs statcheck per record and rejects a three-string record (convM2D.c), failing the whole directory read; dirstat never checks convM2D's return, which is why the three-string shape hid until ps's dirreadall"
    - "TAP transcription for RED evidence (05-06/05-07's established shape): libtest has no --format=tap on this toolchain; the record's output field transcribes the real run verbatim-true"

key-files:
  created:
    - ../r9/port/tests/devproc_test.rs
    - ../r9/port/tests/05-09-red-evidence.json
    - .planning/phases/05-shell-boot-one-command-integration/05-09-drive-r9.log
  modified:
    - ../r9/port/src/devproc.rs
    - ../r9/port/src/devcons.rs
    - ../r9/port/src/namec.rs
    - ../r9/port/src/devmnt.rs
    - ../r9/aarch64/src/proc.rs
    - ../r9/aarch64/src/exec.rs
    - ../r9/aarch64/src/syscall.rs
    - ../r9/aarch64/src/main.rs
    - ../r9/port/tests/devmnt_test.rs
    - ../r9/HowItWorks.md
    - build/aarch64/drive-r9.expect
    - .gitignore
    - .planning/phases/05-shell-boot-one-command-integration/deferred-items.md
    - .planning/WINDOWS.md

key-decisions:
  - "the qid re-encoding (pid*8 + slot: note 1, status 2, args 3, ctl 4; root 0) supersedes 04-08's pid*2/pid*2+1 — qids are transient kernel internals (vers pinned 0), so the re-encoding changes no observable behavior; the note PATH's walk+write semantics are byte-identical (Pitfall 5, proven by the virtnotes profile)"
  - "status's zero-filled fields (the six time fields + memory) are the plan-sanctioned no-analog arms — the field COUNT and order are the ps.c tokenizer contract; name/state are real, basepri/pri report the recorded ctl value"
  - "ctl accepting + storing 'pri N' (Proc.pri) is the plan's honest degradation — no scheduler priority exists this era; unknown verbs error LOUDLY (the donor procctl's Ebadarg shape)"
  - "rfork inherits pri but NOT argv (a fork child's /proc/N/args reads EMPTY until its own exec — the plan's truthful-args contract; the donor newproc shape)"
  - "the mount-redirect fid-clone (devmnt::clone_chan) is the era's answer to the shared-fid aliasing the gate surfaced — the donor's incref/decref fid lifecycle arrives with the fid-refcount era (the 05-07 deferred note); cloning at the redirect is the minimal correct rule"
  - "the input-wedge flake is documented-not-fixed (2 of ~7 early runs; 6 consecutive clean passes since; never reproducing under the tick-side cons watchdog) — this plan's diffs do not touch the cons/wake/note/sched/timer paths; deferred as the phase's top debug candidate for 05-12/05-13's CI leg"

patterns-established:
  - "the devproc host-test idiom: a seeded fake proc table behind the injected hook struct (the devcons_test.rs/devpipe_test.rs scripted-hook precedent) — the port crate stays arch-free and the whole ps surface is host-pinned"
  - "the fid-clone regression pin: open+close a mount redirect, then prove the table's fid survives and a later walk works (devmnt_test.rs's fake-server fid table makes the clunk observable)"

requirements-completed: [INT-01]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "devproc serves the ps(1) surface (D-09): the /proc root directory read emits one stat(5) entry per live pid (decimal-named, DMDIR), a pid dir lists args/ctl/note/status, status emits 12 whitespace fields in devproc.c's order (ps's tokenizer counts/parses them), args serves the exact exec-recorded argv (empty for a never-exec'd pid), ctl parses + records 'pri N' and loudly rejects unknown verbs — and the 04-08 note path is byte-identical"
    requirement: INT-01
    verification:
      - kind: unit
        ref: "cargo test -p port — port/tests/devproc_test.rs (devproc_root_status_args_ctl_and_note_surface): the root listing's per-pid entries + the pid*8 qid encoding + EOF, the pid-dir file set, the 12-field status tokenization (fields 0/1/2 named, 3..12 numeric), the exact argv read + the never-exec'd empty read, the ctl record-then-status round-trip + the loud unknown-verb error, the note post unregressed; the RED evidence verified RED_EVIDENCE_OK (port/tests/05-09-red-evidence.json — target fails at the root-listing read's .expect); the parser walks all FOUR record strings with exact-consumption (the statcheck shape)"
        status: pass
      - kind: e2e
        ref: "05-09-drive-r9.log's ps stage: 'glenda 1 … Blocked init', 'glenda 3 … Blocked paqfs', 'glenda 4 … Blocked rc', 'glenda 8 … Running ps' — 4 pid-and-name lines incl rc (the gate asserts ≥3 + rc); ps exec'd through the /bin union and read /proc end-to-end on the live kernel"
        status: pass
    human_judgment: false
  - id: D2
    description: "Proc records the argv at exec on BOTH paths (install_image is shared by exec_into/exec_replace) and #c/pid makes libc getpid() resolve — init's #p/1/ctl 'pri 10' write lands (its designed-degradation warning is GONE from the boot log)"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "05-09-drive-r9.log: zero 'can't set priority' / 'can't open #p' lines (05-07/05-08's logs carried the warning; this run's is absent — getpid() named init's real pid and devproc ctl accepted the write)"
        status: pass
      - kind: unit
        ref: "the devproc host test's ctl arm: write 'pri 10' → the fake table records pri=10 → the status re-read's fields 10/11 report it"
        status: pass
    human_judgment: false
  - id: D3
    description: "the dispatch-table closure: DISPATCH == the 05-03 measured set ∪ the D-14 named core — verified against 05-03-trace.txt (every measured number has a handler; FAUTH stays loud-None per D-02; REMOVE/WSTAT/FWSTAT/OSEEK were never measured and stay loud-None — the kernel's syscall attack surface is exactly the measured set, T-05-18)"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "05-09-drive-r9.log: the ONLY unimplemented lines are 'syscall 10 unimplemented' ×2 (the designed FAUTH fall-through); zero other walls across the whole sweep"
        status: pass
      - kind: integration
        ref: "the full staged-profile fan (virtexec/virtmulti/virtnotes/virt/virtpanic) PASS on the final tree + builds virtshell/virtexec/default/raspi4b + clippy virtexec + ureg-check 35/35"
        status: pass
    human_judgment: false
  - id: D4
    description: "the full D-09 sweep gate (drive-r9.expect, extended — 05-12's smoke core): boot stages → echo round-trip → ls /bin (rc/ls/cat/echo/ps word-anchored present) → ps (≥3 pid lines incl rc) → cat /lib/namespace ('bind -c #e /env' verbatim) → DEL safety → exit→restart; the capture is retained and no qemu survives the teardown"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "05-09-drive-r9.log — ALL STAGES PASS on the committed tree (r9 1e9323f), exit 0; the teardown leg (pgrep -f qemu-system-aarch64) finds no orphan; six consecutive clean passes on the final tree"
        status: pass
    human_judgment: false
  - id: D5
    description: "the gate-surfaced fixes (the iterate-discipline's yield): namec's mount redirects fid-CLONE the table's DEV_MNT chan (devmnt::clone_chan — the zero-name Twalk) so a redirect's open+close never Tclunks the table's fid (ls /bin then ps exec works), and devproc's stat records carry the FOURTH string (muid) so dirreadall's statcheck accepts them (ps's /proc read)"
    requirement: INT-01
    verification:
      - kind: unit
        ref: "cargo test -p port — devmnt_test.rs's devmnt_mount_redirect_clones_the_table_fid (open+close the redirect, then the table fid survives server-side AND a later walk works) + devproc_test.rs's four-string exact-consumption parser; the full suite green"
        status: pass
      - kind: e2e
        ref: "05-09-drive-r9.log: ls /bin printed the full listing THEN ps exec'd + read /proc (the exact failing order pre-fix); 'ps: empty directory /proc' is gone"
        status: pass
    human_judgment: false

# Metrics
duration: 54 min (incl. the two gate-surfaced fixes + the wedge-flake measurement runs — 11 gate runs total)
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 09: devproc completion + the trace-settled closure + the D-09 sweep gate Summary

**The kernel-side surface the milestone needs is complete and proven interactively: devproc serves ps(1)'s real data source (the /proc root directory read lists live pids; per-pid status emits the 12-field line ps's tokenizer parses; args serves the exec-recorded argv; ctl accepts + records "pri N"), Proc records the argv at every exec, #c/pid makes getpid() resolve so init's pri-10 write lands silently, and the dispatch table is closed against the 05-03 measured set (FAUTH alone stays loud-None, by D-02 design). The extended drive-r9.expect asserts the FULL D-09 sweep green on this host — echo round-trip, ls /bin's five subset names, ps's four pid-and-name lines (init/paqfs/rc/ps), cat /lib/namespace's known line, DEL safety, exit→restart — with the capture retained and zero qemu orphans. The gate's iterate-discipline caught two real integration bugs on the way: namec's mount redirects bare-dup'd the mount table's DEV_MNT fid (ls /bin's open+close Tclunked it out from under ps's exec — fixed with the fid-clone redirect), and the devices' stat records lacked the fourth (muid) string that dirreadall's statcheck demands (ps's /proc read failed "empty directory" — devproc fixed, the other devices' latent shape deferred).**

## Performance

- **Duration:** 54 min
- **Started:** 2026-11-03T14:17:09Z
- **Completed:** 2026-11-03T15:11:14Z
- **Tasks:** 2
- **Files modified:** 14 (9 r9 source + 1 r9 test modified + 2 r9 test/evidence created + 1 r9 docs; harvey: 1 driver + .gitignore + the evidence log + deferred-items + WINDOWS)

## Accomplishments

- **Task 1 (TDD): devproc is the real ps(1) data source** (r9 RED `7774e83`, GREEN `fde0146`): the /proc root is a directory now — its read emits one stat(5) wire entry per live (non-Moribund) pid, decimal-named (ps's `chdir("/proc")` + `open(".")` + dirreadall shape); a pid dir lists args/ctl/note/status; `status` emits 12 whitespace-separated fields in devproc.c's order (name user state + six zero times + zero memory + basepri/pri — count and order are ps.c's tokenizer contract, the zero-filled fields the plan's sanction); `args` serves the argv recorded AT EXEC (proc.rs's new `Proc.argv` — NUL-joined, ARGV_CAP 256 with a documented truncation rule; exec.rs's install_image fills it on BOTH exec paths; a never-exec'd pid reads EMPTY, never garbage), space-joined per the donor's procargs; `ctl` parses + records "pri N" (the plan's honest degradation — `Proc.pri`, rfork-inherited, exec-preserved; unknown verbs are loud). The qid scheme is re-encoded pid*8+slot (documented, never-collide); the note path is byte-identical (Pitfall 5 — the virtnotes profile stays green). The proc-table data crosses via the injected ProcHooks struct (list/info/set_pri — the set_post_hook idiom extended). Host tests pin the whole surface over a seeded fake proc table; RED evidence verified RED_EVIDENCE_OK. Also landed: **#c/pid** (devcons's Qpid via an injected current-pid hook — the phase's deferred item, this plan's wiring: getpid() resolves, so init's `#p/1/ctl` write names the REAL pid and the designed-degradation warning is GONE from the boot log), **namec's "." accept** (the devwalk Accept arm after the mount consultation — ps's open(".") resolves to the devproc union head), and the **dispatch-table closure** (verified against 05-03-trace.txt: every measured number has a handler; REMOVE(25)/WSTAT(44)/FWSTAT(45)/OSEEK(16) were never measured → loud-None; FAUTH(10) stays loud-None per D-02 — the syscall.rs header records it).
- **Task 2 (SWEEP GATE): the full D-09 sweep passes interactively on darwin** (harvey `ca6fd79d3`): drive-r9.expect gains the sweep between the echo round-trip and the 05-08 DEL/exit stages (kept last — they reset the shell state): `ls /bin` (five word-anchored presence assertions), `ps` (≥3 pid-and-name lines, one rc — the capture shows init/paqfs/rc/ps), `cat /lib/namespace` (the literal "bind -c #e /env"). The stage proc reads expect_out via a global (Tcl scoping caught by the first run). The capture moved to build/aarch64/drive-r9.log (the plan's named artifact; .gitignore follows). Evidence retained at the phase dir; provably no qemu orphan. Six consecutive clean passes on the final tree.
- **The gate's iterate-discipline yield — two real integration bugs fixed** (r9 `1e9323f`): (1) **mount-table fid aliasing**: namec's redirects bare-dup'd the mount table's DEV_MNT chan, SHARING its server-side fid — ls /bin's open+close of the redirect Topen'd then Tclunked the TABLE's fid, and every later walk from it (ps's exec) Rerror'd (paqfs: a clunked fid's busy==0 → Enotexist → "namec: does not exist"). The redirects (the final-component arm, the "." arm, sys_create's dot arm) now fid-CLONE via the new `devmnt::clone_chan` (the zero-name Twalk idiom — the donor mntwalk's nwname==0 arm), with a host regression pin. (2) **the stat-record muid**: the stat(5) records carried THREE strings; libc's dirreadall runs statcheck per record, which walks FOUR (name/uid/gid/muid — convM2D.c) and rejects the short record, failing ps's /proc read with "empty directory /proc". devproc's records now carry muid (glenda); the host test's parser pins the four-string exact-consumption shape. (The same three-string shape is latent in devcons/devroot/devenv/devsrv/devdup listings — no dirreadall consumer this era; deferred with the fix named.)

## Task Commits

Each task was committed atomically (two-repo convention — r9 commits land on `main` of `requaos/r9`; harvey commits on master):

1. **Task 1 RED: failing devproc ps-surface test + evidence record** — r9 `7774e83` (test)
2. **Task 1 GREEN: devproc completion + Proc argv/pri + #c/pid + namec "." + dispatch closure** — r9 `fde0146` (feat)
3. **Gate-surfaced fixes: mount-table fid aliasing (fid-clone redirect) + the stat-record muid** — r9 `1e9323f` (fix)
4. **Task 2: the sweep driver + the evidence log + .gitignore** — harvey `ca6fd79d3` (feat)
5. **HowItWorks.md material-change section (the header's mandate)** — r9 `69f4843` (docs)

TDD gate sequence holds: RED (test `7774e83`) precedes GREEN (feat `fde0146`), both present; no REFACTOR commit (the GREEN code was fresh-written and clippy-clean — tdd.md: commit only on change). Verified: `git log --oneline -E --grep="^test\((0*5)-(0*9)\):"` → 7774e83; `--grep="^feat\((0*5)-(0*9)\):"` → fde0146 (+ ca6fd79d3 harvey-side).

**Plan metadata:** (the harvey docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS + deferred-items + WINDOWS.md)

## Files Created/Modified

- `../r9/port/src/devproc.rs` — the 05-09 surface: the re-encoded qid scheme, the extended walk/open/stat, the root + pid-dir listings, the status/args reads, the ctl write, the ProcHooks injection (the note path untouched)
- `../r9/port/tests/devproc_test.rs` — the host suite over the seeded fake proc table (the whole ps surface + the note unregression + the four-string record pin)
- `../r9/port/tests/05-09-red-evidence.json` — the RED evidence record (RED_EVIDENCE_OK)
- `../r9/port/src/devcons.rs` — #c/pid (QID_PID) + the injected current-pid hook
- `../r9/port/src/namec.rs` — the "." accept arm + redirect_clone (the fid-clone redirect) + the final-redirect fix
- `../r9/port/src/devmnt.rs` — Mnt::clone_fid (the zero-name Twalk) + clone_chan
- `../r9/aarch64/src/proc.rs` — Proc.argv + Proc.pri fields, record_argv, the proc_list_live/proc_info/proc_set_pri hooks, pri inheritance at rfork
- `../r9/aarch64/src/exec.rs` — install_image records the argv at every exec
- `../r9/aarch64/src/syscall.rs` — pread's DEV_PROC dispatch to devproc::read; sys_create's dot redirect fid-clones; the header's measured-set closure note
- `../r9/aarch64/src/main.rs` — the hook registrations (devproc proc hooks + devcons pid hook, under dev_exec, not virt-gated)
- `../r9/port/tests/devmnt_test.rs` — the fid-aliasing regression pin
- `../r9/HowItWorks.md` — the 05-09 section (mandated)
- `build/aarch64/drive-r9.expect` — the D-09 sweep (the 05-12 smoke core)
- `.gitignore` — the capture rename (drive-r9.log)
- `.planning/phases/05-shell-boot-one-command-integration/05-09-drive-r9.log` — the committed gate evidence
- `.planning/phases/05-shell-boot-one-command-integration/deferred-items.md` — two new entries (the muid shape in the other devices; the input-wedge flake)
- `.planning/WINDOWS.md` — the ledger entries (the two deferrals; two accidental duplicates waived)

## Decisions Made

- **The qid re-encoding** (pid*8 + slot) supersedes 04-08's pid*2/pid*2+1 — transient kernel internals, no observable change; note's walk+write semantics byte-identical (Pitfall 5).
- **Status zero-fills** the six time fields + memory (the plan's sanction — count and order are the contract); name/state real; basepri/pri report the recorded ctl value.
- **ctl "pri N" is accepted + recorded** (Proc.pri) — the plan's honest degradation; unknown verbs loud (donor Ebadarg shape).
- **rfork inherits pri, NOT argv** (truthful-args: a pre-exec child reads EMPTY).
- **The fid-clone redirect** is the minimal correct rule for the shared mount-table fid (the donor's full incref/decref fid lifecycle arrives with the fid-refcount era).
- **The input-wedge flake is documented-not-fixed** — 2 of ~7 early runs wedged at a prompt read; six consecutive clean passes since; never reproduced under the tick-side cons watchdog; this plan's diffs don't touch the cons/wake/note/sched/timer paths. Deferred as the phase's top debug candidate (the D-14 known-flake policy class) for 05-12/05-13's CI leg.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Wiring] namec had no "." accept — ps's open(".") could never resolve**
- **Found during:** Task 1's GREEN wiring walk (the plan's key_link names the path: "ps → chdir(/proc) + open(".")")
- **Issue:** namec passed "." components to the device walk (a walk error everywhere) — the donor's devwalk Accept arm (9k dev.c) was never ported
- **Fix:** "." is accepted AFTER the mount consultation (resolving to the union head — ps's chdir+open(".") lands on the devproc attach); ".." stays unhandled (no measured caller — loud)
- **Files modified:** `../r9/port/src/namec.rs` (beyond the plan's file list — the plan's key_link sanctions the wiring)
- **Verification:** the gate's ps stage (ps reads /proc end-to-end)
- **Committed in:** `fde0146`

**2. [Rule 2 - Missing critical] #c/pid absent — init's pri write named pid 0 (the phase's deferred item, assigned to this plan)**
- **Found during:** Task 1 (the plan's must_have: "the designed-degradation line leaves the boot log" — impossible with getpid()=0)
- **Issue:** libc getpid() reads #c/pid; devcons had no pid file, so init's ctl path named #p/0/ctl (the walk rejects pid 0 → the warning stayed)
- **Fix:** the deferred item's prescribed shape — devcons's Qpid file via a devproc-style injected current-pid hook (proc::current_pid exists since 05-08; registered under dev_exec, not virt-gated)
- **Files modified:** `../r9/port/src/devcons.rs`, `../r9/aarch64/src/main.rs`
- **Verification:** 05-09-drive-r9.log has ZERO pri-10 warnings (05-07/05-08's logs carried it)
- **Committed in:** `fde0146`

**3. [Rule 1 - Bug, gate-surfaced] mount-table fid aliasing — a redirect's open+close Tclunked the TABLE's server-side fid**
- **Found during:** Task 2's gate — ls /bin printed, then ps's exec failed "namec: does not exist" (the TEMP instrumentation pinned it: Twalk from fid 5 → paqfs Enotexist ×2 — fid 5 was the /bin union head's fid, clunked by ls's redirect)
- **Issue:** namec's final-component redirect bare-dup'd the mount table's DEV_MNT chan (shared fid); the redirect's open+close lifecycle terminated the table's fid for every later walk
- **Fix:** devmnt::clone_chan (the zero-name Twalk fid-clone idiom — the donor mntwalk's nwname==0 arm) + namec's redirect arms (the final component, the "." arm, sys_create's dot arm) clone DEV_MNT heads instead of bare-dup'ing
- **Files modified:** `../r9/port/src/devmnt.rs`, `../r9/port/src/namec.rs`, `../r9/aarch64/src/syscall.rs`, `../r9/port/tests/devmnt_test.rs` (the regression pin)
- **Verification:** the pin test (the table's fid survives the redirect's lifecycle; a later walk works) + the gate's ls→ps order passes
- **Committed in:** `1e9323f`

**4. [Rule 1 - Bug, gate-surfaced] the stat(5) records lacked the fourth (muid) string — dirreadall's statcheck rejected them**
- **Found during:** Task 2's gate — ps printed "empty directory /proc" (dirreadall returned -1)
- **Issue:** statcheck walks FOUR length-prefixed strings (convM2D.c) and rejects a three-string record; the devices' write_stat_record shape carried name/uid/gid only (dirstat never checks convM2D's return — why it hid until ps's dirreadall)
- **Fix:** devproc's records carry muid (glenda); the host test's parser walks all four strings with exact-consumption (the statcheck shape pinned)
- **Files modified:** `../r9/port/src/devproc.rs`, `../r9/port/tests/devproc_test.rs`
- **Verification:** the pin + the gate's ps stage
- **Committed in:** `1e9323f`
- **Scope note:** the same three-string shape is latent in devcons/devroot/devenv/devsrv/devdup listings (pre-existing, no dirreadall consumer this era) — logged to deferred-items.md + WINDOWS.md, NOT fixed here (the scope boundary)

**5. [Process note] the TEMP debug instrumentation came and went uncommitted**
- The fid-aliasing bisect used TEMP prints (devmnt's Twalk fid/name + the Rerror tag + sys_exec's namec-failure path print) and the wedge measurement used a tick-side cons watchdog (fire when RX-pending persists >2000 ticks + a proc-table cons-state dump) — all removed before any commit; the committed tree carries none of it.

---

**Total deviations:** 4 (1 Rule 3 wiring + 1 Rule 2 missing-critical — both the plan's own key_links/deferred assignment; 2 Rule 1 gate-surfaced bugs) + 1 process note
**Impact on plan:** Every deviation is the gate's iterate-discipline working as designed (run the gate, let the loud line name the gap). No scope creep: the fid-clone is the minimal aliasing fix, the muid is one string, #c/pid is the deferred item's own prescription, and the "." accept is the donor's devwalk arm.

## Issues Encountered

- **The input-wedge flake (documented, not fixed):** the extended driver wedged in 2 of its first ~7 runs — mid-sweep, at a prompt read, the typed line never completes (run A: `ls /bin` never started after the echo stage; run B: `echo r9delok` never answered after the DEL re-prompt; no fork/sched activity follows, no overrun/panic/concurrent-rpc line). Six consecutive clean passes followed — including four with the tick-side cons watchdog in place, which never fired. This plan's diffs do not touch the cons read/wake, note, scheduler, or timer paths; the shape sits in 05-08's cons-input wake machinery or below (qemu stdio/pty delivery, PL011 RX IRQ, the tick wake). The instrumented runs suggest a ~1-in-6 mid-run rate on the extended (six-command) driver — the D-14 known-flake policy class. **Deferred as the phase's top debug candidate** (deferred-items.md + WINDOWS.md), with the reproduction recipe recorded (the tick-side watchdog distinguishes bytes-pending-no-wake from bytes-never-arrive; then the 05-08 register-dump recipe). 05-12/05-13's CI leg should re-run or root-cause it before the milestone's CI claim.
- **expect 5.45/Tcl scoping caught the first sweep run:** `expect_out` is set in the CURRENT scope — inside the `stage` proc it was local, so the sweep's post-stage buffer analysis saw nothing. The proc now declares `global expect_out`.
- **The r9 devshell is the gate's host environment** (qemu + dtc + coreutils' timeout; expect is the bare host's /usr/bin/expect) — run from the harvey cwd via `nix develop /Users/neilskinner/code/r9 -c …` (the plan's verify command verbatim, wrapped).

## User Setup Required

None - no external service configuration required. (Driving the gate by hand needs qemu-system-aarch64 + fdtput + expect on PATH — the r9 devshell provides qemu/dtc/coreutils; expect is the bare host's /usr/bin/expect; the exact invocation is the plan's verify command under `nix develop ../r9 -c`.)

## Known Stubs

- **FAUTH(10) stays loud-None** — the designed D-02 path (the log's two `syscall 10 unimplemented` lines are the design working).
- **REMOVE(25)/WSTAT(44)/FWSTAT(45)/OSEEK(16) stay loud-None** — never measured by the 05-03 artifact (execinit's chmod is unchecked on the donor too); the dispatch table is exactly the measured set (the plan's must_have: "the set is exactly the trace plus nothing").
- **status's time/memory fields are zero-filled** (the plan's sanction — no analog this era; count and order are the contract).
- **ctl "pri N" is recorded, not scheduled-by** (the plan's honest degradation — no scheduler priority this era).
- **The three-string stat shape in devcons/devroot/devenv/devsrv/devdup listings** (deferred-items.md + WINDOWS.md #21) — latent, no dirreadall consumer this era; the fix (add muid) is named.
- **The input-wedge flake** (deferred-items.md + WINDOWS.md #20) — the top debug candidate for 05-12/05-13.
- 05-04..05-08's known stubs are unchanged except as resolved here: the devproc ps surface + ctl + #c/pid are LANDED (05-07's deferral closed), the pgrp-lock-across-blocking-walk hazard stays deferred (the gate's chain is strictly sequential), the exit-sweep deferral persists (the Moribund-slot pinning — the gate's procs are well under it), no input echo (documented delta).

## Threat Flags

None beyond the plan's register. (T-05-17 held: status/args reads resolve per-pid by exact pid; the argv is kernel-recorded at exec, never read back from user memory at read time; the write side stays note/ctl-only with the addressed-pid rule — the ctl hook resolves the ADDRESSED pid, no broadcast. T-05-18 held: no new syscall handlers — the dispatch table is exactly the measured set, everything unmeasured stays loud-None; the plan's leftover set needed nothing new. T-05-SC held: zero package installs. The one new read surface — the /proc listings + status/args — is the plan's own modeled surface, world-readable per the Plan 9 model named in the register.)

## Next Phase Readiness

- **05-10/05-11 (composition):** the kernel stream is feature-complete for the milestone — the composition apps consume the proven kernel ELF + image + qemu argv. NOTE for the same-wave 05-10: this plan touched none of its scope (../r9/flake.nix, ../r9/xtask/ are untouched).
- **05-12 (the packaged smoke):** grows from drive-r9.expect verbatim — the sweep IS the smoke's core; the capture discipline (fresh log, process-group teardown, no-orphan proof) is established. The input-wedge flake is its headline risk (documented with the reproduction recipe).
- **05-13 (CI/reproducibility):** the gate runs green on darwin repeatedly; the linux CI leg inherits the flake note.
- r9 HEAD: `69f4843` (main; code at `1e9323f`). Harvey HEAD: `ca6fd79d3` + the metadata commit.
- The staged-profile fan + host tests + clippy + ureg-check are green on the final tree (through the fixes).

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: r9 port/src/{devproc,devcons,namec,devmnt}.rs, aarch64/src/{proc,exec,syscall,main}.rs, port/tests/{devproc_test.rs, 05-09-red-evidence.json, devmnt_test.rs}, HowItWorks.md; harvey build/aarch64/drive-r9.expect, .gitignore, .planning/phases/05-shell-boot-one-command-integration/{05-09-SUMMARY.md, 05-09-drive-r9.log, deferred-items.md}, .planning/WINDOWS.md — all FOUND on disk.
- Commits: r9 7774e83 (RED test), fde0146 (GREEN feat), 1e9323f (gate-surfaced fixes), 69f4843 (docs); harvey ca6fd79d3 (Task 2 + evidence) — all FOUND in the respective git logs; RED precedes GREEN (the TDD gate sequence holds; no REFACTOR — the GREEN code was fresh-written and clippy-clean).
- Plan-level <verification> re-run on the final tree: (1) host tests — `cargo test -p port` all 13 test binaries green (the new devproc suite + the devmnt fid-clone pin; the RED evidence verified RED_EVIDENCE_OK); (2) builds virtshell/virtexec/default/raspi4b + clippy virtexec + ureg-check 35/35 green; (3) the staged-profile fan — virtexec/virtmulti/virtnotes/virt/virtpanic all PASS (the note-only surface byte-identically working — Pitfall 5); (4) the sweep gate — ALL STAGES PASS on the committed tree (05-09-drive-r9.log: banner/paqfs/stacksmoke/prompt/echo/ls-subset/ps-lines/cat-line/del-reprompt/del-answer/restart), exit 0, `pgrep -f qemu-system-aarch64` after the run empty; six consecutive clean passes on the final tree.
- Acceptance criteria re-run at final state: Task 1 (devproc serves the ps surface — dir reads, 12-field status, truthful args, parsed ctl — with host tests; note unregressed; Proc records argv on both exec paths; the dispatch table is exactly the measured set, unmeasured numbers loud-None; everything green) — PASS. Task 2 (the full D-09 sweep asserted green from a single run — echo, ls /bin's 5 subset names, ps's ≥3 pid lines incl rc, cat /lib/namespace's known line, DEL safety, exit→restart; the capture retained; no qemu leaks) — PASS.
