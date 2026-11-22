---
phase: 04-r9-userspace-exec-harvey-r9-abi
plan: 08
subsystem: kernel/notes
tags: [r9, aarch64, notes, devproc, notify, noted, notify-boundary, sc4-gate, phase-gate, cross-repo-abi, d16]

requires:
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 04)
    provides: Proc note_queue/notify_handler seams, PROC_TABLE, NOTE_QUEUE_CAP
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 05)
    provides: the trap_return tail + EL0-entry machinery (the delivery boundary's resume path)
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 06)
    provides: syscall dispatch table + handlers + validate_user_range, devcons/devroot devices, the exec profile harness
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 07)
    provides: CURKSTACK/CURFRAME preemption machinery (the IRQ-path delivery resolves CURFRAME), swtch-based blocking syscalls, exit_tail's wait records
provides:
  - devproc-lite (port/src/devproc.rs): /proc/N/note only — walk a numeric pid then the literal note name, write-only open, write appends ERRMAX-bounded NUL-terminated bytes to the addressed pid's queue via the arch-registered post hook (T-04-17); libc postnote(PNPROC) works unmodified
  - NOTIFY(28): handler VA validated 4-aligned + inside the proc's own text segment; NOTED(29): NCONT/NRSTR restore the delivered frozen 280-byte Ureg over the live frame (kernel-tracked note_ureg, donor up->noteureg), NDFLT exits with the note message, NSAVE loud-unimplemented; NOTED's success takes dispatch's no-writeback special return (the donor dosyscall NOTED case)
  - deliver_notes at BOTH return-to-EL0 boundaries (post-syscall + post-IRQ, the IRQ path preemption-safe via proc::current_frame), the donor notify() stack construction verbatim (256 preserve + frozen 280-byte Ureg copy + msg slot + SP+8/SP+16 arg homes), T-04-16 okaddr port (validate-then-write + forced residency via fault::ensure_stack_resident)
  - exec clears the note machinery / rfork inherits the notify handler (donor sysexec/newproc semantics)
  - smokenote + init's final staged job (D-04 complete: hello + smokefork + smokenote + postnote + await all three) + config_virtnotes boot-test profile (the SC#4 gate)
  - 04-EVIDENCE.md: the phase's canonical evidence bundle with the r9 SHA pin (D-13) and the SC#1-4 mapping table
affects:
  - Phase 5 (real /bin/init stands on devproc-lite's note channel + the notes machinery + the proven five-profile harness; staged-init pattern grows into namespace/auth)
actuals:
  tokens: 33563
  tasks: 3
  commits: 6
  commits_r9: 4
  commits_harvey: 2
  plan_head_before_r9: fdac9fe1cbff0a06090f6928ce7216c04050124e
  plan_head_after_r9: b4f858abe015a66f403290580970c94a5e57a835
  plan_head_before_harvey: e867e1a1e67704ec6ba0943bb9a58df14334e2b2
  sc4_evidence: "04-08-sc4-boot.log — 7 sched: switch lines, full D-16 marker set, zero panic/Unhandled/note-timeout/unimplemented-syscall lines; 3/3 consecutive virtnotes PASS at the gate commit"

tech-stack:
  added: []
  patterns:
    - "note delivery at the return-to-EL0 boundary: one deliver_notes check on BOTH return paths (post-dispatch + post-IRQ); the IRQ path resolves CURFRAME (proc::current_frame) because a tick may preempt mid-trap — deliver into the frame the tail actually restores, for the proc that is actually current"
    - "device-post via injected hook: port can't see the arch's PROC_TABLE, so devproc-lite's write calls a registered fn pointer (Console::set_uart-style inversion), registered once at boot"
    - "NOTED's no-writeback return: dispatch skips frame.x0 = result when n == NOTED && result != -1 (the handler rewrote the frame; error paths left it untouched and DO write back) — the donor dosyscall NOTED special case"
    - "user-stack construction with kernel writes: validate the WHOLE write window user-writable BEFORE any write, then force residency (in-segment-but-unmapped pages would EL1h-abort mid-delivery — a user-triggerable kernel panic), never a partial write (T-04-16)"
    - "7c + cross-boundary flags: a plain static load (even volatile) is cached in a register across a spin loop by 7c — a flag the note handler sets must be read through a real C call (never inlined), or the store is never observed"

key-files:
  created:
    - ../r9/port/src/devproc.rs
    - ../r9/aarch64/lib/config_virtnotes.toml
    - build/aarch64/smokenote.c
    - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-EVIDENCE.md
    - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-08-sc4-boot.log
  modified:
    - ../r9/port/src/lib.rs (mod devproc) + dat.rs (DEV_PROC=2 convention)
    - ../r9/aarch64/src/syscall.rs (NOTIFY/NOTED handlers + dispatch special return + /proc route + DEV_PROC pwrite/pread arms + exit_tail factored from sys_exits)
    - ../r9/aarch64/src/trap.rs (deliver_notes + the two boundary call sites + UREG_SIZE/NCONT..NRSTR consts)
    - ../r9/aarch64/src/proc.rs (note_ureg + last_note fields, post_note, current_frame accessor, rfork notify inheritance)
    - ../r9/aarch64/src/fault.rs (ensure_stack_resident)
    - ../r9/aarch64/src/exec.rs (exec clears the note machinery — donor sysexec)
    - ../r9/aarch64/src/main.rs (register the devproc post hook in exec_bootstrap)
    - ../r9/xtask/src/main.rs (notes profile: INITRD_FILES + smokenote, marker set extension, BootProfile::Notes, negative-scan note-timeout/unimplemented legs)
    - ../r9/HowItWorks.md (the notes + devproc-lite tour — file-header mandate) + README.md (five boot-test profile rows)
    - build/aarch64/init.c (rfork C + ordered postnote + await ×3) + stage-build.sh (STAGE_SRCS += smokenote)

key-decisions:
  - "The IRQ-return delivery boundary resolves proc::current_frame() (CURFRAME), not the trap-entry frame: the tick may preempt mid-trap, so the proc about to return to EL0 is the CURRENT one and its frame is CURFRAME's — the plan's deliver_notes(frame) letter is honored with the frame the tail actually restores (syscall path: no preemption is possible IRQ-masked, so `frame` == CURFRAME there)"
  - "Delivery residency: the donor's okaddr check is segment membership; r9 additionally forces the write window's stack pages resident (fault::ensure_stack_resident) before writing — an in-segment-but-unmapped user page would EL1h-abort the kernel mid-delivery (a user-triggerable panic), which T-04-16's 'never a kernel write to a bad user VA' forbids"
  - "sys_noted validates the resume pc/sp BEFORE the 280-byte restore copy (pc 4-aligned + inside text; sp 8-aligned + user-writable) so a bad saved frame leaves the live frame untouched and the -1 writeback stays truthful — strictly safer than the donor's restore-then-validate-and-kill"
  - "exec clears notify_handler + the note queue + note_ureg (donor sysexec freenotes semantics — a stale handler VA would redirect delivery into the NEW image, T-04-17); rfork INHERITS the handler (donor newproc)"
  - "The note-post ordering without a rendezvous: init's CPU-bound spin orders the postnote AFTER child C's exec+notify (a note delivered to a handlerless/execing process kills it — donor pexit semantics), and doubles as the D-13 preemption window; the failure mode on a lost race is loud (the smokenote markers go absent, failing the profile)"
  - "smokenote's flag read goes through a real call (seen()): the SC#4 gate caught 7c caching a plain static load in a register across the spin loop (volatile included) — the handler's store was never observed, so the resumed spin timed out"

patterns-established:
  - "the notes profile extends the multi profile's marker set by config-NAME selection (kernel cfg byte-identical; only xtask assertions differ) — the idiom now covers three exec-family profiles (virtexec/virtmulti/virtnotes)"
  - "the always-on negative scan gained note-timeout + unimplemented-syscall legs for every non-panic profile (D-14's discipline: a measured-set gap fails loudly, never silenced)"
  - "phase evidence consolidation: one 04-EVIDENCE.md mapping every success criterion to quoted log lines + the D-13 r9 SHA pin + the regression matrix with commands"

requirements-completed: [R9-02]

coverage:
  - id: T1
    description: "devproc-lite (/proc/N/note only — numeric-pid walk then the literal note name, write-only open, write resolves the addressed pid via the post hook; scope guard clean even in comments) + NOTIFY (text-segment-validated handler VA) + NOTED (NCONT/NRSTR restore of the frozen 280-byte Ureg, NDFLT exit-with-message, NSAVE loud-unimplemented, no-writeback special return) + deliver_notes at both return-to-EL0 boundaries with the donor stack construction and the validate-then-write residency-forced window; all four pre-existing profiles + raspi3b/raspi4b builds + clippy green (pure no-regression — no notes exist yet)"
    requirement: "R9-02"
    verification:
      - kind: integration
        ref: "builds virtexec/default(raspi3b)/raspi4b + clippy virtexec green at c464ea2; plan grep chain PASS (pub mod devproc; note present; zero ctl/status/notepg substrings in devproc.rs code lines — the stricter whole-file scan is also zero; notify_handler/deliver_notes/NCONT/NDFLT present); boot-test virtexec/virtmulti/virt/virtpanic all PASS at c464ea2; re-confirmed virtnotes+virtexec PASS at the final HEAD dac3b33"
      status: pass
    human_judgment: false
  - id: T2-SC4-GATE
    description: "SC#4: the D-16 smoke narrative runs green under preemption from EL0 — smokenote (notify + bounded spin), init's final job (rfork A/B/C, ordered postnote via unmodified libc postnote, await all three), config_virtnotes profile with the extended marker set + the note-timeout/unimplemented negative legs; the evidence log committed; regression fan green"
    requirement: "R9-02"
    verification:
      - kind: integration
        ref: "stage-build PASS (init/smokefork/smokenote compiled, per-binary R_MAGIC 00008c47 verified); initrd bake (bootfs.paq sha256 c04421d2…); boot-test virtnotes PASS 3/3 consecutive at the gate commit (fae19fc) + 1/1 at the final HEAD (dac3b33) — 04-08-sc4-boot.log carries: hello aarch64 harvey, smokefork: fd ok, smokefork: child running, smokefork: await: 5 0 0 0 \"kid\", smokenote: got note smoke, smokenote: resumed, init: reaped ×3 (incl. 4 0 0 0 \"note-ok\"), init: smoke complete, all user processes exited, 7 sched: switch lines; explicit grep legs PASS: zero panic/Unhandled lines, zero note-timeout, zero 'syscall N unimplemented'; regression fan virtmulti/virtexec/virt/virtpanic PASS + raspi3b/raspi4b builds + clippy green at the final state; sys/src/libc untouched across the phase's Harvey commits (verified empty)"
      status: pass
    human_judgment: true
    rationale: "the plan's verify carries a <human-check> — the SC#4 gate's end-of-phase UAT harvest reviews 04-08-sc4-boot.log (the D-16 narrative under visible preemption, panic-free) before Phase 5 planning; all automated legs pass 3/3"
  - id: T3
    description: "04-EVIDENCE.md consolidates the phase's canonical evidence — the exact r9 HEAD SHA (D-13), the SC#1-4 mapping table with quoted log lines, hello sha256, the CTR_EL0 probed value, the D-14 measured set + coverage table, Wave-0 falsification results, and the regression matrix with commands; HowItWorks.md gains the notes + devproc-lite tour; README.md lists all five boot-test profiles; r9 commits precede the Harvey evidence commit"
    requirement: "R9-02"
    verification:
      - kind: integration
        ref: "verify legs PASS at the final HEAD (b4f858a): 04-EVIDENCE.md carries the live r9 HEAD SHA (grep match) + SC#1/SC#2/SC#3/SC#4 rows + hello sha256 1c89e099… + ctr_el0; HowItWorks.md carries ABI.md + userspace/demand tour text; README.md carries virtnotes; both docs tracked (git ls-files --error-unmatch); commit order demonstrates r9-first (c464ea2 → fae19fc → dac3b33 → b4f858a, then Harvey 6f3c04a36 recording the final SHA)"
      status: pass
    human_judgment: false

duration: 40min
completed: 2026-11-02
status: complete
---

# Phase 04 Plan 08: Notes + devproc-lite + the SC#4 Phase Gate Summary

**SC#4 is satisfied with execution evidence, closing the phase: r9 now implements Plan 9 notes end-to-end — devproc-lite serving exactly `/proc/N/note` (libc `postnote(PNPROC, pid, msg)` works unmodified through open/write/close), NOTIFY/NOTED joining the dispatch table (with the donor's no-writeback special return), and note delivery at the return-to-EL0 boundary with the donor's stack construction verbatim — proven by the full D-16 smoke narrative under tick preemption: init rforks hello + smokefork + smokenote, posts `smoke` to the spinning child, the kernel delivers at the child's tick-return boundary (pc redirected to the notify handler, the saved 280-byte Ureg + the note string on the user stack), the handler prints and `noted(NCONT)` resumes the spin, the child exits `note-ok`, and init reaps all three — zero panic lines, 7 preemption switches, across three consecutive gate runs. The phase's canonical evidence is consolidated in 04-EVIDENCE.md pinned to r9 HEAD `b4f858abe…` (D-13 ordering: all r9 commits landed first).**

## Performance

- **Duration:** 40min (started 2026-11-02T18:43:15Z)
- **Tasks:** 3/3
- **Commits:** 4 r9 (`c464ea2` Task 1, `fae19fc` Task 2, `dac3b33` Task-3 docs, `b4f858a` style fix) + 2 Harvey (`c72d06314` Task 2, `6f3c04a36` Task-3 evidence) + the metadata commits carrying this file
- **Verification runs:** 4 build configs + clippy (two states), boot-test virtnotes ×4 (3 at the gate commit + 1 at final HEAD), virtmulti ×2, virtexec ×3, virt ×2, virtpanic ×2, ~8 debugging gate runs (below)

## Accomplishments

- **Task 1 (r9 `c464ea2`) — the notes machinery, no-regression-green.** devproc-lite (`port/src/devproc.rs`): attach → the /proc root, walk a numeric pid component then the literal `note` name (everything else a loud walk error), write-only open on a note chan, write decodes the pid from the qid and posts ERRMAX-bounded NUL-terminated bytes through the arch-registered hook (`proc::post_note` — the Console::set_uart-style inversion, since port can't see PROC_TABLE). The comment-wording constraint is honored: the file never names the omitted devproc file set anywhere (code or comments), so the scope guard stays truthful under any scan widening. NOTIFY(28) validates the handler VA 4-aligned and inside the proc's own text segment (T-04-17); NOTED(29) implements NCONT/NRSTR (restore the delivered frozen 280-byte Ureg over the live frame, resume pc/sp validated pre-copy), NDFLT (exit with the note message via the factored `exit_tail`), NSAVE (loud-unimplemented + -1) — with dispatch's NOTED no-writeback special return. `deliver_notes` runs on BOTH return-to-EL0 boundaries; the IRQ path resolves CURFRAME (`proc::current_frame()`) so a mid-trap preemption delivers to the proc actually about to run EL0. The delivery construction is the donor's notify() verbatim (256 preserve, the frozen 280-byte copy, the msg slot, the SP+8/SP+16 arg homes, pc→handler, LR=0), guarded by validate-then-write + forced residency (T-04-16). Gate: four profiles + two raspi builds + clippy all green (pure no-regression).
- **Task 2 (r9 `fae19fc`; Harvey `c72d06314`) — SC#4 GATE GREEN.** smokenote (notify + bounded spin + loud note-timeout ceiling), init's final staged job (rfork A/B/C; the postnote ordered after C's exec+notify by a CPU-bound spin that doubles as the D-13 preemption window; await ×3), and the virtnotes profile (name-selected, kernel cfg byte-identical to virtexec/virtmulti; marker set = multi ∪ {smokenote: got note, smokenote: resumed}; negative scan + note-timeout/unimplemented legs). The evidence log (committed as 04-08-sc4-boot.log) shows the textbook narrative: delivery at a tick boundary with the interrupted pc inside main's spin, the handler's print, the noted restore back into the spin, `smokenote: resumed`, `note-ok`, all reaped, all-exited — 7 `sched: switch` lines, zero panic/Unhandled/note-timeout/unimplemented lines. 3/3 consecutive PASS at the gate commit, re-PASSed at the final HEAD.
- **Task 3 (r9 `dac3b33` + style fix `b4f858a`; Harvey `6f3c04a36`) — the phase gate.** 04-EVIDENCE.md maps SC#1 (two-layer ABI gate + negative control) / SC#2 (hello chain + sha256 1c89e099…) / SC#3 (demand paging + ctr_el0 0x8444c004 + the ttbr0 teardown proof) / SC#4 (the quoted smoke chain + 7 switches) to evidence, carries the D-14 measured set + coverage table, the Wave-0 falsification results, and the full regression matrix — pinned to the live r9 HEAD (grep-verified). HowItWorks.md gained the notes + devproc-lite tour (file-header mandate); README.md lists all five boot-test profiles.
- **The gate caught two real bugs** (see Deviations): the /proc route's off-by-one (the smoke's postnote failed loudly), and 7c's register caching of the spin flag (the resumed loop never observed the handler's store).

## Task Commits

| Task | Repo | Commit | Files |
|------|------|--------|-------|
| 1 | r9 | `c464ea2` | port/src/devproc.rs (new), lib.rs, dat.rs, aarch64/src/{syscall,trap,proc,fault,exec,main}.rs |
| 2 | r9 | `fae19fc` | xtask/src/main.rs, aarch64/lib/config_virtnotes.toml (new), aarch64/src/syscall.rs (the off-by-one fix) |
| 2 | harvey | `c72d06314` | build/aarch64/smokenote.c (new), init.c, stage-build.sh, 04-08-sc4-boot.log (new) |
| 3 | r9 | `dac3b33` | HowItWorks.md, README.md |
| 1 (style) | r9 | `b4f858a` | aarch64/src/proc.rs (post_wait_record line-join fix — whitespace only) |
| 3 | harvey | `6f3c04a36` | 04-EVIDENCE.md (new) |
| metadata | harvey | (final commits) | .planning SUMMARY/STATE/ROADMAP/REQUIREMENTS |

**r9 HEAD at evidence time (D-13 ordering — all r9 commits landed first):** `b4f858abe015a66f403290580970c94a5e57a835` (whitespace-only style fix atop docs-only `dac3b33` atop the gate-tested `fae19fc` + `c464ea2`; the gate ran on the exact committed tree and was re-run green at the final HEAD).
**Evidence-run bootfs.paq sha256:** `c04421d2ae4f924c6671f8736f7a696e430a712ba7ba2581653c827b65cd286c` (per-bake digest — mkpaqfs embeds a time field).

## Files Created/Modified

**r9:** `port/src/devproc.rs` (new, ~145 lines), `port/src/lib.rs`, `port/src/dat.rs`, `aarch64/src/syscall.rs`, `aarch64/src/trap.rs`, `aarch64/src/proc.rs`, `aarch64/src/fault.rs`, `aarch64/src/exec.rs`, `aarch64/src/main.rs`, `xtask/src/main.rs`, `aarch64/lib/config_virtnotes.toml` (new), `HowItWorks.md`, `README.md`.

**Harvey:** `build/aarch64/smokenote.c` (new), `build/aarch64/init.c`, `build/aarch64/stage-build.sh`, `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-EVIDENCE.md` (new), `04-08-sc4-boot.log` (new).

## Decisions Made

See key-decisions in the frontmatter: the CURFRAME-resolved IRQ delivery boundary, the forced-residency write window, the pre-copy resume validation, exec-clears/rfork-inherits note state, the spin-based post ordering, and the call-based flag read (7c caching).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug found by the SC#4 smoke] /proc route off-by-one in sys_open**
- **Found during:** Task 2 first virtnotes run — init's postnote failed loudly: `init: postnote failed: devproc: not a numeric process name`
- **Issue:** `path[4..]` on "/proc/4/note" yields "c/4/note" ("/proc" is FIVE bytes) — the walk's first component was "c", rightly rejected as non-numeric
- **Fix:** strip the full 5-byte prefix (`&path[b"/proc".len()..]`) before the component walk
- **Files modified:** ../r9/aarch64/src/syscall.rs
- **Verification:** the smoke's postnote then ran clean through devproc-lite (open → write → close → the note queued and delivered)
- **Commit:** fae19fc (r9)

**2. [Rule 1 — Bug found by the SC#4 gate, in the smoke itself] 7c caches the spin flag in a register; the handler's store is never observed**
- **Found during:** Task 2 debugging — with the note correctly posted and delivered (handler printed `smokenote: got note smoke`, `noted(NCONT)` restored the spin-loop frame), the process still exited `note-timeout`
- **Issue:** the compiled spin loop (`if(notedflag)` on a plain static — volatile included) keeps the flag in a callee register across the loop and never reloads it on the spinning path (verified against `7c -S`): the reload instruction sits on the flag-set branch path. A note handler runs on the SAME thread of control (interrupt-style), so the resumed loop never saw the store
- **Fix:** read the flag through a real C call (`seen()`) — Plan 9 compilers never inline C functions, so every iteration reloads the memory location
- **Files modified:** build/aarch64/smokenote.c
- **Verification:** 3/3 consecutive virtnotes PASS with `smokenote: got note` + `smokenote: resumed` + `note-ok` in the await record; zero `note-timeout`
- **Commit:** c72d06314 (harvey)

**Plan-supporting additions (in-plan intent):** `proc::current_frame()` accessor (the IRQ boundary's CURFRAME resolution — the plan's "deliver_notes(frame)" honored with the frame the tail restores); `fault::ensure_stack_resident` (the T-04-16 write window's residency half — the donor's okaddr assumes the kernel fault path can grow the stack; r9's fault handler is lower-EL-only, so residency is forced before the write); Proc's `note_ureg`/`last_note` fields (the donor's up->noteureg/lastnote — libc's noted(action) passes no pointer, so the saved-ureg source is kernel-tracked); exec-clears-notes + rfork-inherits-handler (donor sysexec/newproc semantics, T-04-17); the devproc post hook (port/arch inversion); the virtnotes profile plumbing (BootProfile::Notes + the marker-set extension idiom). Debugging-round instrumentation (kernel dbg prints + tick counters) was added, used to pin both bugs, and fully removed before the Task-2 commit — the committed tree carries zero debug output.

**Total deviations:** 2 auto-fixed (one found by each gate layer) + the plan-supporting additions. No Rule 4 (architectural) items.
**Impact on plan:** none to intent — every must-have truth, acceptance criterion, and plan-level verification item passes at the final state.

## Known Stubs

None introduced. NSAVE(2) is loud-unimplemented BY DESIGN (the plan names it; the smoke's notejmp path uses NCONT). The `#c/pid` open remains the soft-fail from 04-06 (OQ-2 watch item): exits()'s atexit machinery calls getpid() → open("#c/pid") → -1 (devroot has no such path), tolerated (atol of a zeroed buffer → pid 0, no atexit registrants in the smoke) — observed in the log for every process, benign, and unchanged by this plan.

## Issues Encountered

- The off-by-one and flag-caching bugs (deviations 1–2) cost ~7 instrumented debugging gate runs; the decisive evidence was the delivered frame's identity (type=8/x0=-1/pc at a stub return — proving delivery at the open()-return boundary of exits()'s getpid) plus the `7c -S` loop disassembly.
- Preemption nondeterminism moves the delivery boundary run to run (tick vs syscall return) — both paths are exercised across runs; the marker-not-order assertion model (D-13) is exactly right for it.
- The note-post ordering without a rendezvous primitive is calibration-based (init's spin), per the plan's smoke shape; a lost race fails LOUDLY (the smokenote markers go absent → the profile fails), never silently.

## Threat Flags

None new beyond the plan's register: T-04-16 landed as designed (validate-then-write the whole window + forced residency; unwritable/misaligned/handlerless → loud kill with the note printed; never a partial write, never a kernel write to a bad user VA). T-04-17 landed (the write resolves only the addressed pid against PROC_TABLE; NOTIFY's handler VA is confined to the caller's own text segment; exec clears the handler so a stale VA can never redirect a new image). T-04-SC: no package installs this plan (no new crates).

## User Setup Required

None.

## Next Phase Readiness

**Phase complete, ready for next step.** Phase 4's four success criteria all carry execution evidence (04-EVIDENCE.md): SC#1 static ABI enforcement, SC#2 hello on r9, SC#3 demand-paged first process, SC#4 the rfork/notes/fd-ops smoke from EL0 under preemption, panic-free. Phase 5 inherits: the staged-init pattern to grow into real /bin/init (namespace, auth, boot rc), a proven five-profile boot-test harness, an ABI contract neither repo can drift from silently, the notes machinery (devproc-lite + NOTIFY/NOTED + boundary delivery), and the swtch-based blocking-syscall path. Flagged assumption carried from the plan: R9-02 remains probe-unclassified — acceptance derived from ROADMAP SC#4 + CONTEXT D-16 and the 04-01 trace artifact; the verifier should treat the probe row as a flagged assumption, not a resolved edge. Watch item (OQ-2): the smoke set's exits() path reads #c/pid (getpid) and tolerates the soft failure; if a future binary's behavior demands it, devcons gains one static pid file.

## Self-Check: PASSED

- Key files exist: r9 `port/src/devproc.rs` ✓ `aarch64/lib/config_virtnotes.toml` ✓; Harvey `build/aarch64/smokenote.c` ✓ `init.c` ✓ `stage-build.sh` ✓; `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-EVIDENCE.md` ✓ + `04-08-sc4-boot.log` ✓ (committed, 515 lines).
- Commits: r9 `git log --grep="04-08"` → `c464ea2` (Task 1) + `fae19fc` (Task 2) + `dac3b33` (Task-3 docs) ✓; Harvey → `c72d06314` (Task 2) + `6f3c04a36` (Task 3) ✓ (plus the metadata commit carrying this file).
- 04-EVIDENCE.md's recorded r9 SHA == the live r9 HEAD (`b4f858abe015a66f403290580970c94a5e57a835`) ✓ (grep-verified).
- Acceptance criteria re-run at final HEADs: Task 1 (4-config builds + clippy green; grep chain PASS incl. the scope guard; four profiles PASS) ✓; Task 2 (stage-build PASS + per-binary R_MAGIC; initrd bake; virtnotes PASS ×3 at the gate commit + ×1 at final HEAD; all explicit log grep legs PASS; regression fan virtmulti/virtexec/virt/virtpanic + raspi3b/raspi4b + clippy green; sys/src/libc untouched) ✓; Task 3 (SHA pin + SC rows + hello sha256 + ctr_el0 greps PASS; docs greps PASS; commit order r9-first) ✓.
- Plan-level verification at final HEADs: SC#4 gate satisfied (full D-16 marker set + preemption evidence + empty negative scans); regression fan green; evidence SHA pin == live HEAD; docs tour landed in the final r9 commit.

---
*Phase: 04-r9-userspace-exec-harvey-r9-abi*
*Completed: 2026-11-02*
