---
phase: 19-cpu-service-drawterm-rio-desktop
plan: 03
subsystem: kernel
tags: [exec, hash-bang, shebang, argv, syscall, dispatch, remove, wstat, fwstat, statcheck, devcap, r9, x86_64, boot-profile]

# Dependency graph
requires:
  - phase: 19-cpu-service-drawterm-rio-desktop/19-02
    provides: the TCP/timer substrate kernel tree (df9ada8) this plan builds on, and the donor-trace research (15-06) bounding the wstat/remove measured set
provides:
  - r9 execs hash-bang scripts with the donor's exact argv semantics (sysproc.c:282-324): #! tokens lead (argv[0] substituted by the script's elem), the script path lands after them, the caller's argv follows with its argv[0] dropped — listen's blind execl of a service file becomes the interpreter with -R (CPU-01's kernel half)
  - the recursion cap (depth 2, loud) — the donor's implicit one-hop protection made explicit — and the loud Ebadexec-class rejects (unterminated line, empty tokens, MAX_ARGS never silently truncated)
  - the WSTAT/FWSTAT/REMOVE syscall surface: DISPATCH carries named handlers at REMOVE=25, _WSTAT=26, _FWSTAT=27, WSTAT=44, FWSTAT=45, each routing to per-device semantics (devcap's caphash remove arm — the first consumer) or a loud device-level refusal naming the gap (CPU-03's syscall half)
  - the donor statcheck port (valid_stat_wire) gating every wstat buffer BEFORE any device routing (T-19-06)
  - the §15 cpu image's live proof legs: cpurc is now a REAL #! rc script carrying the SHEBANG-OK (the #! token reaching /amd64/bin/echo — the argv order live from ring 3) and REMOVE-OK (rm #¤/caphash through the ported devcap arm + test -e absence) markers, with the q35cpu gate grepping both directions (OK markers present, FAIL words absent)
affects: [19-04 (devssl rides this kernel tree), 19-05 (pin advance over these kernel bytes — 0a39318..f691914), 19-07 (the seventh contract's real cpurc + listen/tcp17010 consume the #! pre-pass and the syscall arms; 19-07 co-declares CPU-03), 19-08 (exportfs-through-listener composed proof)]

# Actuals (#2632) — pairs with the plan's estimate (52000 tokens, low confidence).
# chars/4 over the realized diff: r9 59,060 + harvey 10,495 chars.
actuals:
  tokens: 17388
  tasks: 3
  commits: 5   # MEASURED: git rev-list --count df9ada8..f691914 (r9 only; harvey's 83ae39392 + the docs commit ride beside)

# Tech tracking
tech-stack:
  added: []   # zero external packages — in-tree donors only (T-19-SC honored)
  patterns:
    - "pre-pass + namespace seam: the kernel-side #! algorithm lives in exec.rs as a pure, host-pinnable half (shebang_pass) plus a loop (exec_with_shebang) that resolves the interpreter through a caller-supplied seam (exec_backing_for) — the exec entry owns the algorithm, the syscall layer owns namec"
    - "loud-None to named-arm routing: per-device match fns (remove_chan/wstat_chan) with the loud refusal naming the gap as the default arm — a device port joins as one match line"
    - "ring-3 leg fixtures as #! scripts: the boot-profile script IS the first proof (cpurc execs through the kernel pre-pass itself), with pass/fail marker pairs grepped both directions"

key-files:
  created: []  # evidence artifacts beside this SUMMARY: evidence/19-03-t1-red-evidence.json + 19-03-t1-red-raw-libtest.json + 19-03-t2-red-evidence.json + 19-03-t2-red-raw-libtest.json
  modified:
    - ../r9/x86_64/src/exec.rs
    - ../r9/x86_64/src/syscall.rs
    - ../r9/xtask/src/main.rs
    - ../r9/x86_64/lib/config_q35cpu.toml
    - build/x86_64/mkimage.sh
    - build/x86_64/stage-build.sh

key-decisions:
  - "The donor argv order is [elem, #!tokens[1..], file, caller.argv[1..]] — sysproc.c:308-309 REPLACES progarg[0] (the interpreter path) with elem ('replace $0 with the original path's name') and :446-447 drops the caller's argv[0]; the plan's behavior-block bracket shorthand named [interp, script-path, caller-argv...] but its own prohibition clause ('No argv convention of our own: the donor's sysproc.c:282-324 shape') makes the donor the authority — implemented donor-verbatim and pinned as such"
  - "All five slots carry REAL semantics including _WSTAT(26)/_FWSTAT(27): this tree's 9k donor retires them as 'old wstat system call - recompile' error stubs (systab.c:93-94), but the plan's must-have truth mandates 'each routes to real per-device semantics... never a silent no-op' — 26/27 alias the live name/fd bodies (the plan's own action text describes 26/27 as the stat-buffer/fd forms). Harvey's libc emits wstat/fwstat stubs against 44/45, so those are the load-bearing pair exportfs actually calls"
  - "The #! line bound is the donor's 32 (sizeof(Exec), sysproc.c:293's MIN(sizeof(Exec), hdrsz)) — not the 40-byte Hdr read; the MAX_ARGS check is LOUD where the donor's tokenize silently drops past nelem(progarg)=17 (T-19-05's token cap, the plan's Test-5 'never truncates silently')"
  - "A head shorter than 2 bytes is NOT a script (no marker can exist) — read_aout's own unchanged short-read error owns that shape, keeping the plain-a.out else-branch byte-identical (the must-have's 'exactly as today')"
  - "The remove leg's target is #¤/caphash, not a paqfs file: devroot is read-only by design and devcap's caphash arm is the ONLY ported remove arm (the plan's own 'devcap's ported arm is the first consumer') — the donor's 'rm a staged file' read as the served device file the ported arm owns, with the loud refusal arm as the other proven half"

patterns-established:
  - "Union-visible staging: namespace's `bind /root/$cputype/bin /bin` REPLACES /bin, so files that must appear under /bin stage under rc/bin and surface through the `bind -a /root/rc/bin /bin` append (the donor k10root service shape)"
  - "paqfs enforces the exec bit on OEXEC opens — fixture scripts staged by mkimage heredocs need explicit 755 (the 644 default refuses, measured live)"

requirements-completed: [CPU-01]  # CPU-03 co-declared by 19-07 — the shared-ID gate blocks it until 19-07's SUMMARY exists (this plan delivered its syscall half)

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The #! exec pre-pass (CPU-01): donor argv order [elem, tokens[1..], file, caller.argv[1..]], two-token (-R) order, no-newline/empty-token loud rejects, MAX_ARGS loud bound, plain-a.out unchanged, the interpreter image installs with the recorded donor argv + name=the script leaf, depth-2 recursion cap"
    requirement: CPU-01
    verification:
      - kind: unit
        ref: "r9 x86_64 bin tests (cargo test -p x86_64 --bins -- exec): 19 passed — the six 19-03 pins (shebang_one_token_builds_the_donor_argv_order, shebang_flag_tokens_precede_the_script_path, shebang_negative_shapes_are_loud_rejects, plain_aout_head_is_not_a_script, shebang_argv_growth_respects_max_args, exec_with_shebang_installs_the_interpreter_image, shebang_recursion_is_capped_at_two_with_a_loud_reject) + the 13 pre-existing exec/syscall siblings"
        status: pass
      - kind: integration
        ref: "RED evidence evidence/19-03-t1-red-evidence.json classified RED_EVIDENCE_OK (19 discovered / 13 passed / 6 planned failures) then GREEN — the test->feat commit pair 0a39318/d188171"
        status: pass
    human_judgment: false
  - id: D2
    description: "The five DISPATCH arms (CPU-03's syscall half): Some(handler) at 25/26/27/44/45, no collateral arming (9/10/30/31/36 stay None), the statcheck port's reject shapes, remove routing to devcap's ported arm (Ok(0) under eve) with loud refusals elsewhere, WSTAT/FWSTAT behavioral pins (validate-then-refuse, malformed fails validstat FIRST, fd form over an open fd)"
    requirement: CPU-03
    verification:
      - kind: unit
        ref: "cargo test -p x86_64 --bins -- syscall: 14 passed — cpu03_slots_carry_named_handlers, other_loud_none_slots_stay_unarmed, valid_stat_wire_ports_the_donor_statcheck, remove_round_trips_through_the_devcap_arm_and_refuses_unported, wstat_over_an_unported_device_refuses_loudly_never_zero, remove_syscall_dispatches_and_refuses_loudly, wstat_syscall_validates_then_refuses, fwstat_syscall_over_an_open_fd_refuses_loudly + the 6 pre-existing syscall pins"
        status: pass
      - kind: integration
        ref: "RED evidence evidence/19-03-t2-red-evidence.json classified RED_EVIDENCE_OK (14 discovered / 8 passed / 6 planned failures) then GREEN — the test->feat commit pair 69a1a5b/20c704c"
        status: pass
    human_judgment: false
  - id: D3
    description: "The live boot-profile legs + battery + push: the §15 cpurc #! script fixture (cpurc: RAN proves #! at the cpurc point), the shebang leg (SHEBANG-OK — the #! token reaching /amd64/bin/echo is the donor argv order live from ring 3), the remove leg (REMOVE-OK — rm #¤/caphash through the ported devcap arm + test -e absence), zero FAIL words; q35cpu green 4x KVM, q35shell/q35auth control legs green 2x each, battery (check+clippy+port 151 + x86_64 103) green 2x, r9 pushed df9ada8..f691914 (PUSHED-CLEAN)"
    requirement: CPU-01
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --config q35cpu — boot-test: PASS with SHEBANG-OK + REMOVE-OK + cpurc: RAN in target/boot-test-q35cpu.log and zero FAIL lines; the plan's verify command verbatim: EXECSYSCALL-LEGS-GREEN and BATTERY-PUSHED"
        status: pass
    human_judgment: false

# Metrics
duration: 47 min
completed: 2026-11-15
status: complete
commits_note: "r9 plan_head_before=df9ada8 plan_head_after=f691914 (both == origin/main, PUSHED-CLEAN); harvey plan_head_before=b8c544f44"
---

# Phase 19 Plan 03: exec/syscall half of the cpu service Summary

**r9 execs hash-bang scripts with the donor's exact argv semantics (tokens lead with argv[0]=elem, script path, caller argv minus its argv[0]) and the WSTAT/FWSTAT/REMOVE surface moves from loud-None to five named arms routing to per-device semantics (devcap's caphash remove first) or loud refusals — proven live from ring 3 by the §15 legs (cpurc itself a #! script now): SHEBANG-OK (the token reaching the interpreter) and REMOVE-OK, zero FAIL, battery green twice, pushed**

## Performance

- **Duration:** 47 min
- **Started:** 2026-11-14T23:39:55Z
- **Completed:** 2026-11-15T00:27:07Z
- **Tasks:** 3
- **Files modified:** 6 (4 in r9, 2 in harvey) + 4 evidence artifacts

## Accomplishments

- **Task 1 (TDD, RED->GREEN):** the `#!` pre-pass — `shebang_pass` (exec.rs) is the donor algorithm of record as a pure, host-pinnable half: the 32-byte MIN(sizeof(Exec), hdrsz) line bound, tokenize on the marker bytes, the script path appended, progarg[0] substituted by elem, the caller's argv[0] dropped (sysproc.c:446-447), loud Ebadexec-class rejects (unterminated line, empty tokens), and MAX_ARGS loud where the donor silently drops. `exec_with_shebang` loops it in front of exec_replace: the header read precedes parse_aout's magic check, the interpreter resolves through the namespace seam (`exec_backing_for`, factored from sys_exec — normal namec, T-19-05), recursion capped at depth 2 with the donor's implicit one-hop protection made explicit. The plain-a.out else-branch is byte-identical (hdrsz<2 is not a script; read_aout's own errors own the non-#! shapes).
- **Task 2 (TDD, RED->GREEN):** five named DISPATCH arms — REMOVE(25)/_WSTAT(26)/_FWSTAT(27)/WSTAT(44)/FWSTAT(45) — with `valid_stat_wire` (the donor statcheck port: n==2+size, n>=STATFIXLEN 49, four string counts consume exactly the rest; "malformed stat buffer" = the donor's Ebadstat) firing BEFORE any device routing (T-19-06), `remove_chan` routing DEV_CAP to devcap's ported caphash arm with the loud refusal naming the gap elsewhere, `wstat_chan` refusing loudly for every device this era (no device ports a wstat arm — rio has none, sam uses remove only, the 15-06 measured set; exportfs relays the refusal to its client). Devices gained arms: devcap remove (the consumer). Devices refusing loudly: every other device, both syscalls.
- **Task 3:** the §15 cpu image's live proof — the 19-01 marker binary replaced by a REAL #! rc script (cpurc itself is the first live #! proof), carrying the shebang leg (a service-shaped script under rc/bin/service with `#!/amd64/bin/echo SHEBANG-OK` — the token on serial is the argv-order proof end-to-end) and the remove leg (`rm '#¤/caphash'` through the ported arm + `test -e` absence), both with named FAIL markers; the q35cpu gate's stage terminal and finish post-pass grep both directions (OK present, FAIL absent, zero init-loop). The profile gains the `cap` dev key. rm + test join the stage-build chain set. Battery green twice; r9 pushed df9ada8..f691914.

## Task Commits

Each task was committed atomically (cross-repo: all code lands in r9 `main`, push-before-pin; the harvey flake.lock advance is 19-05's, never here):

1. **Task 1 RED: the #! pins + pre-change stubs** - r9 `0a39318` (test)
2. **Task 1 GREEN: the #! pre-pass implementation** - r9 `d188171` (feat)
3. **Task 2 RED: the DISPATCH-arm pins + stubs** - r9 `69a1a5b` (test)
4. **Task 2 GREEN: the five arms + statcheck + routing** - r9 `20c704c` (feat)
5. **Task 3: the q35cpu gate legs + cap key** - r9 `f691914` (feat)
6. **Task 3: the §15 script fixtures + rm/test staging** - harvey `83ae39392` (feat)

**Plan metadata:** (this SUMMARY + STATE/ROADMAP commit, below)

## Files Created/Modified

- `../r9/x86_64/src/exec.rs` - SHEBANG_LINE_BOUND/MAX_SHEBANG_DEPTH, ShebangExec, shebang_pass (the donor pure half), exec_with_shebang (the loop + seam), 7 new test fns
- `../r9/x86_64/src/syscall.rs` - REMOVE/WSTAT_/FWSTAT_/WSTAT/FWSTAT numbers, valid_stat_wire (statcheck port), remove_chan/wstat_chan routing, sys_remove/sys_wstat/sys_fwstat + wstat_stage_args, the five DISPATCH arms, sys_exec's exec_backing_for refactor, 8 new test fns + the user-hook test seam
- `../r9/xtask/src/main.rs` - Q35_CPU_SHEBANG_OK/REMOVE_OK markers + the FAIL-word fast-fail, the Q35CpuRc stage terminal, the X64Cpu finish both-direction post-pass
- `../r9/x86_64/lib/config_q35cpu.toml` - dev gains "cap" (the remove leg's device)
- `build/x86_64/mkimage.sh` - §15: the cpurc #! script fixture + the shebangecho service script (755, rc/bin/service union staging), rm/test staging + manifest, the section doc/PASS line re-keyed
- `build/x86_64/stage-build.sh` - rm + test join the chain sweep set (+ droppings)

## Decisions Made

See key-decisions. Additionally: the wstat length cap (n > 2+u16::MAX rejects with the same malformed message BEFORE the copy — statcheck's own bound, no extra string); the donor's validstat name-entry follow-on check (validname on <=63-byte names) documented as server-protection nicety, not ported; the mkimage bake venue re-confirmed as the flake-app PATH shape (the 18-06 environment fact — inside `nix develop` the auth section's TARG resolver still loses hsecstored to the vendored awk; the sanctioned PATH bakes all 9 contracts green).

## TDD Gate Compliance

Tasks 1 and 2 are `tdd="true"` with the RED->GREEN cycle and the commit-scope contract honored (test(...) precedes feat(...), both in r9): Task 1 `0a39318`/`d188171`, Task 2 `69a1a5b`/`20c704c`. Both RED captures are libtest-JSON runs mechanically converted to TAP (19-02's transform) and classified `RED_EVIDENCE_OK` — Task 1: 19 discovered / 13 passed / 6 planned failures (`evidence/19-03-t1-red-evidence.json`); Task 2: 14 / 8 / 6 (`evidence/19-03-t2-red-evidence.json`). Semantic assessment in each record: every failure is an assertion failure for the planned behavior against the honest pre-change stubs (no sniff / no routing / no validation / unarmed table); the first Task-1 capture exposed a harness bug (a 40-byte head pad under a 112-byte line) and the first Task-2 capture a slice-length typo — both fixed BEFORE the evidence runs so no failure is a fixture fault. One pin was redesigned during Task 1's GREEN phase: the MAX_ARGS pin originally grew argv from the #! line's tokens, which trips the donor's 32-byte line bound (correct behavior); the redesign grows past 32 from the CALLER's argv, which is also the shape copy_exec_args can actually stage. Plan-level `workflow.tdd_mode` is false (config), so no plan-type gate applies; the task-level cycles are complete. No REFACTOR phase needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] sys_exec refactor into exec_backing_for (Task 1)**
- **Found during:** Task 1 (the pre-pass wiring)
- **Issue:** The plan's files list named exec.rs only, but the #! pre-pass must resolve the interpreter through the namespace at the exec entry — the sniff cannot live in exec.rs without a seam at the syscall layer
- **Fix:** sys_exec's namec/open/backing block factored into `exec_backing_for(proc, path)` (syscall.rs), passed to exec_with_shebang as the resolver seam — the algorithm stays in exec.rs, namec stays in syscall.rs
- **Files modified:** ../r9/x86_64/src/syscall.rs
- **Verification:** all exec/syscall tests green; the 95->103 bin battery green
- **Committed in:** r9 0a39318 (part of Task 1's RED commit — the stub entry needed the call site)

**2. [Rule 3 - Blocking] stage-build.sh rm/test extension (Task 3)**
- **Found during:** Task 3 (the first bake)
- **Issue:** The remove leg's `rm` and `test -e` had no producer — the §15 chain set was ls/cat/echo/ps only
- **Fix:** rm + test join the chain sweep set (mkmany link-only targets, same as the rest) + the droppings list; mkimage's CHAIN input check + §15d staging + manifest gain both
- **Files modified:** build/x86_64/stage-build.sh, build/x86_64/mkimage.sh
- **Verification:** stage-build STAGE-BUILD-OK with both harvested; the bake manifest asserts both staged
- **Committed in:** harvey 83ae39392

**3. [Rule 1 - Bug] The service script staged under /bin/service was invisible (Task 3)**
- **Found during:** Task 3 (the first gate run: `/bin/service/shebangecho: namec: does not exist`)
- **Issue:** namespace's `bind /root/$cputype/bin /bin` is a PLAIN bind (REPLACE), so the paq-local /bin/service directory is shadowed; the donor's own service files never live there
- **Fix:** staged under rc/bin/service (the donor k10root shape) — the `bind -a /root/rc/bin /bin` append surfaces it at /bin/service/shebangecho through the union
- **Files modified:** build/x86_64/mkimage.sh
- **Verification:** the second gate run resolved the path (the failure moved from namec to the open)
- **Committed in:** harvey 83ae39392

**4. [Rule 1 - Bug] paqfs refused the service script's OEXEC open (Task 3)**
- **Found during:** Task 3 (the second gate run: `devmnt: server error: permission denied`)
- **Issue:** The mkimage heredoc's default 644 mode — paqfs checks the exec bit on OEXEC opens; rc's exec of the service file needs +x (cpurc itself only ever needs READ: rc interprets it as its script file, which is why cpurc ran on the first attempt)
- **Fix:** chmod 755 both fixture scripts, with the measured finding recorded in the section comment
- **Files modified:** build/x86_64/mkimage.sh
- **Verification:** the third gate run green (SHEBANG-OK present, both legs' verdicts)
- **Committed in:** harvey 83ae39392

---

**Total deviations:** 4 auto-fixed (2 blocking, 2 bugs)
**Impact on plan:** All four were prerequisites for the plan's own verifies passing; no scope creep. The argv-order reading (donor-verbatim over the plan's bracket shorthand) and the 26/27 arming (plan text over the 9k donor's retired stubs) are recorded as key-decisions, not deviations — each follows the plan's own must-have/prohibition clauses.

## Issues Encountered

- The first two gate runs failed for the staged-fixture reasons above (the union shadowing, then the exec bit) — each fixed at the source and the failure class recorded in the bake comments; the third run green, then three more green runs for the twice-plus margin.
- The mkimage bake must run with the flake-app PATH shape (GNU awk first, plan9port last): inside `nix develop` the auth section's TARG resolver loses hsecstored (the 18-06 environment fact, re-confirmed verbatim this session). The bakes for this plan ran `env PATH=/run/current-system/sw/bin:... plan9port-last PLAN9=... sh build/x86_64/mkimage.sh` — all 9 contracts PASS.

## Known Stubs

None - no stubs, placeholders, or unwired data paths were introduced. (wstat_chan's refusal-for-every-device is the designed loud boundary, not a stub: no device ports a wstat arm in the measured set, every refusal names the gap, and exportfs relays it to its client — the composed exportfs consumer lands with 19-07/19-08 per the plan's own verification note.)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- CPU-01's kernel half and CPU-03's syscall half are landed, pinned, live-proven, and pushed (r9 at `f691914` == origin/main); 19-04 (devssl) and 19-07 (the seventh contract: real cpurc + listen/tcp17010 consuming exactly this #! surface and these syscall arms) build on this tree
- Kernel bytes changed: the boot-iso FOD hash re-stales structurally (the standing 13-06/14-07 discipline) — 19-05's re-wire-then-pin pass covers it; the flake.lock r9 pin is NOT advanced here (per the cross-repo discipline)
- CPU-03 stays open at the requirements level: 19-07 co-declares it (the shared-ID gate blocks until 19-07's SUMMARY exists) — this plan delivered its syscall half; 19-07's listener/exportfs bring the composed half
- The q35cpu gate now carries the leg vocabulary (SHEBANG-OK/REMOVE-OK/zero-FAIL) — 19-07's real cpurc replaces the fixture body and the gate re-keys its expectations there

## Self-Check: PASSED

- Modified files exist on disk: ../r9/x86_64/src/exec.rs, syscall.rs, ../r9/xtask/src/main.rs, ../r9/x86_64/lib/config_q35cpu.toml, build/x86_64/mkimage.sh, build/x86_64/stage-build.sh FOUND
- Task commits exist and are pushed: r9 0a39318, d188171, 69a1a5b, 20c704c, f691914 (== origin/main, PUSHED-CLEAN) FOUND; harvey 83ae39392 FOUND
- RED evidence exists and classified: evidence/19-03-t1-red-evidence.json + evidence/19-03-t2-red-evidence.json (both RED_EVIDENCE_OK) FOUND
- Acceptance criteria re-verified at close-out: exec.rs's pre-pass branches before the magic check (exec_with_shebang's loop precedes exec_replace's read_aout/parse_aout); the six pin classes + five-slot DISPATCH pin green (95->103 bin tests); the gate log shows SHEBANG-OK + REMOVE-OK + cpurc: RAN with zero FAIL (EXECSYSCALL-LEGS-GREEN), q35cpu PASS x4 KVM, q35shell/q35auth PASS x2, battery twice, BATTERY-PUSHED

---
*Phase: 19-cpu-service-drawterm-rio-desktop*
*Completed: 2026-11-15*
