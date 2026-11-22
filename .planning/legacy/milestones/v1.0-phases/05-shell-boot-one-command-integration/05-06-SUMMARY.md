---
phase: 05-shell-boot-one-command-integration
plan: 06
subsystem: kernel
tags: [r9, aarch64, devpipe, devdup, devsrv, fd-group, rfork, RFFDG, RFCFDG, pipe, sys_pipe, sys_dup, sys_create, sys_fd2path, sys_seek, cold-pointer, residency, connectembed, paqfs, tdd, qemu]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-04)
    provides: the namespace core (namec registry + union walk + pgrp mount table) the '|'/'d'/'s' devices register into; the checked open/stat dispatch shape; the build-order lesson (the gated config builds LAST); eager data+bss at exec (the residency class's first half)
  - phase: 05-shell-boot-one-command-integration (05-05)
    provides: devenv's EnvGroup + the injected-hook idiom (devdup/devsrv/devpipe hooks mirror it); the seeded env (nobootprompt=embed drives the method pick); the hardened /rootfs.img node (paqfs's byte source); the pipe-wall gate this plan demolishes
  - phase: 05-shell-boot-one-command-integration (05-03)
    provides: 05-03-trace.txt — the measured set this plan implements from (pipe ×1, dup ×9, create ×67, fd2path ×3, seek ×72)
provides:
  - port::devpipe — the Arc-shared blocking pipe device (registry '|'): two cross-connected bounded queues (9k devpipe.c semantics), block-on-empty-read via the 04-07 machinery through injected hooks, wake-on-write, EOF-after-drain on the last write-end close, 'write on closed pipe', qreopen, 4 KiB named-overflow
  - port::fdgrp FdGroup<N> + proc's Arc<Lock<FdGroup>> — real rfork fd-group semantics per fork(2)/sysproc.c (NEITHER share / RFFDG copy / RFCFDG empty); MAX_PROCS=16 / MAX_FDS=32 with loud exhaustion
  - port::devdup (registry 'd') — per-fd files #d/N resolving the CALLING proc's fd via hook (rcmain's `. -i '#d/0'` contract); port::devsrv (registry 's') — the service registry boot's FATAL srvcreate needs (create + fd-number write binds a reference clone; open returns the posted chan)
  - syscall handlers: sys_pipe, sys_dup, sys_create, sys_fd2path, sys_seek (+ RFNOTEG/RFCFDG accepted in rfork); dat.rs's Chan Arc-carrying pipe variant + copen + the dup reference rule; namec::close_chan/create_chan/walk_one
  - fault::ensure_user_resident — the cold-syscall-buffer class closed (explicit page-in before kernel deref; 04-08's residency generalized; T-05-06 holds)
  - the connectembed GATE: Harvey's real boot(8) pipes, forks, execs /boot/paqfs (which reads+verifies /rootfs.img and parks its io server on the pipe read — serving), then walls loudly at FVERSION(40); zero panic/deadlock
  - evidence: .planning/phases/05-shell-boot-one-command-integration/05-06-boot-r9.log (the gate run)
affects: [05-07 (devmnt's fversion/mount rides the pipe this plan proved; settime's open(#s/boot) resolves devsrv's posted clone; init's setenv/cpenv ride sys_create), 05-08 (cons reads + the namespace-file stanza; rc pipelines ride the fd-group semantics), 05-09 (the smoke's ps/cat sweep), all later plans (the checked-out-chan fd op pattern + close dispatch are now the fd-table contract)]

# Actuals (#2632) — pairs with the plan's estimate (110000 tokens, confidence: low).
actuals:
  tokens: 43841    # MEASURED chars/4: r9 diff 138612 + harvey deferred-items diff 2385 + this SUMMARY 34369 = 175366 chars (the committed 100410-byte gate log is bulk evidence, excluded per the 05-04/05-05 convention); the estimate (110k) overshot ~2.5x
  tasks: 3
  commits: 5       # MEASURED: r9 git rev-list --count 628d612..5af3fe1 = 5; harvey 0 pre-metadata (the docs commit rides on top, per the 05-04/05-05 convention)
plan_head_before: {r9: "628d612", harvey: "8e3546583"}
plan_head_after: {r9: "5af3fe17deb5d7e908e2a86d81cbc368f2f9ae16", harvey: "8e3546583 (pre-metadata; the docs commit rides on top, per the 05-04/05-05 convention)"}

# Tech tracking
tech-stack:
  added: []        # zero new dependencies anywhere (T-05-SC held: no package installs)
  patterns:
    - "the Arc-shared pipe: one Pipe object both ends reference (Arc<Lock<Pipe>> on the chan); the donor's two cross-connected queues (end i reads q[i], writes q[1-i]); open-end counting (qref) rides Chan::dup (a value copy of an OPEN pipe end is a new open reference) and namec::close_chan"
    - "the checked-out-chan fd op pattern: fd_take/fd_store (a blocking pipe op never holds the fd-group lock); fd_ref_clone for reference creation; heap-backed slot Vec (an inline [Option<Chan>; 32] overflows the 16 KiB kstack in the rfork chain — the debug trail of record)"
    - "the block/wake idiom for kernel devices: register the waiter pid under the device lock, drop the lock, block via the injected hook (sched()); write/close takes the waiter list and wakes after the lock drops — un-losable by construction under serialized syscalls"
    - "syscall-buffer residency: validate_user_range THEN fault::ensure_user_resident THEN deref (the 04-08 note pattern generalized; explicit page-in, never a nested fault — T-05-06)"
    - "the gate's iterate-discipline exercised for real: run the gate, let the loud wall name the gap (SEEK 39) or surface the bug (the EL1h cold-buffer abort), implement exactly that, re-run"

key-files:
  created:
    - ../r9/port/src/devpipe.rs
    - ../r9/port/src/fdgrp.rs
    - ../r9/port/src/devdup.rs
    - ../r9/port/src/devsrv.rs
    - ../r9/port/tests/devpipe_test.rs
    - ../r9/port/tests/fdgroup_test.rs
    - ../r9/port/tests/devdup_devsrv_test.rs
    - ../r9/port/tests/05-06-red-evidence.json
    - .planning/phases/05-shell-boot-one-command-integration/05-06-boot-r9.log
  modified:
    - ../r9/port/src/dat.rs
    - ../r9/port/src/lib.rs
    - ../r9/port/src/namec.rs
    - ../r9/port/src/pgrp.rs
    - ../r9/aarch64/src/syscall.rs
    - ../r9/aarch64/src/proc.rs
    - ../r9/aarch64/src/main.rs
    - ../r9/aarch64/src/fault.rs
    - ../r9/HowItWorks.md
    - .planning/phases/05-shell-boot-one-command-integration/deferred-items.md
    - .planning/WINDOWS.md

key-decisions:
  - "the pipe is TWO cross-connected queues in ONE Arc-shared Pipe (9k devpipe.c is the plan's own semantic reference and governs): the plan's 'single shared queue' prose names the shared pipe STATE both ends reference — paqfs's io(1) reads AND writes fd 1, which a single FIFO cannot serve (a server would consume its own replies)"
  - "rfork fd-group semantics follow fork(2)+9k sysproc.c — the plan's bullet inverts RFFDG vs default (NEITHER share / RFFDG copy / RFCFDG empty); the donor governs — the phase's THIRD flag inversion (05-04 RFNAMEG, 05-05 RFENVG precedents). libc fork()=rfork(RFPROC|RFFDG|RFREND)=COPY is exactly what connectembed's child dup/close choreography requires; the staged init's rfork(RFPROC|RFMEM) children now SHARE its fd table (fork(2)) and the fan proves them green"
  - "FdGroup slots are heap-backed (Vec) — the inline [Option<Chan>; 32] materialized ~5.7 KB on the stack in Proc::new and the rfork chain on a 16 KiB kstack overflowed into the proc's root-table page (symptom: stack reads returned UART-register bytes mid-print — 'd'+13×'u'); the debug trail is in the HowItWorks.md section"
  - "fault::ensure_user_resident closes the cold-syscall-buffer class (phase-04 deferred #3's post-exec half): paqfs's emalloc'd heap buffers are brk_-grown demand pages whose first touch was the kernel's pread write — EL1h abort at the gate. Explicit zero-page-in before deref (04-08's T-04-16 pattern generalized); never a nested fault, so T-05-06 holds"
  - "RFNOTEG/RFCFDG accepted in rfork's SUPPORTED set: RFNOTEG is a no-op (per-pid notes have no group channel to split) and paqfs's rfork(RFFDG|RFPROC|RFNAMEG|RFNOTEG) requires it — caught by code-reading paqfs.c before the gate run"
  - "devsrv's open-returns-posted-chan and devdup's resolve are host-test-pinned this plan: the gate's FVERSION wall structurally precedes srvcreate (boot.c:141 fversion, :144 srvcreate — the plan's gate prose inverted that order; the binding verify greps all pass)"

patterns-established:
  - "device close dispatch: namec::close_chan fires when an open chan leaves an fd slot (sys_close, dup-overwrite, install-failure); only devices with close semantics act (devpipe this era)"
  - "the create contract: namec::create_chan is the donor chan.c Acreate's device half (copen-marked open return); sys_create owns the parent walk + the existing-file → OTRUNC-open fallback per OEXCL"
  - "registry-char constants per device (devdup::REGISTRY_RUNE b'd', devsrv::REGISTRY_RUNE b's') keep the plan's grep legs truthful"

requirements-completed: [INT-01]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "devpipe: the Arc-shared cross-connected queue pair (write on one end, read on the other, consume-once), the bounded queue's named atomic overflow, blocking read with scripted wake-on-write and reader-wakes-on-close, write-on-closed-reader, qreopen — host-proven per the plan's behavior bullets"
    requirement: INT-01
    verification:
      - kind: unit
        ref: "cargo test -p port — port/tests/devpipe_test.rs 6/6 (pipe_pair_write_read_round_trip, pipe_full_is_a_named_atomic_error, pipe_reader_eof_after_drain_on_write_close, pipe_write_on_closed_reader_end_errors, pipe_reopen_after_full_close, pipe_blocks_on_empty_and_write_wakes_with_bytes)"
        status: pass
      - kind: e2e
        ref: "05-06-boot-r9.log: boot's pipe(p) succeeds (no 'fatal: pipe'), the child's paqfs serves on its dup'd fds, and the paqfs io child parks on a pipe read (the block path's live exercise)"
        status: pass
    human_judgment: false
  - id: D2
    description: "fd-group semantics + sizing: rfork fd arms per fork(2) (NEITHER share / RFFDG copy / RFCFDG empty), pipe-queue sharing across the copy boundary with refcounted ends, slot mechanics + loud exhaustion, MAX_PROCS=16/MAX_FDS=32; sys_pipe (validated pair write-back) + sys_dup (evict-after-install)"
    requirement: INT-01
    verification:
      - kind: unit
        ref: "cargo test -p port — port/tests/fdgroup_test.rs 3/3 (fd_group_slot_mechanics, fd_group_matrix_share_copy_empty, fd_group_copy_shares_the_pipe_queue)"
        status: pass
      - kind: integration
        ref: "boot-test virtexec/virtmulti/virtnotes PASS through the new fd machinery (init's rfork(RFPROC|RFMEM) children SHARE its table per fork(2)) + the sizing greps land; the gate's connectembed rides sys_pipe/sys_dup/fork live"
        status: pass
    human_judgment: false
  - id: D3
    description: "devdup (#d/N per-fd files via the injected resolver, the donor fdtochan mode check, the listing) + devsrv (create/write/listing; the fd-number write binds an Arc-safe reference clone; open returns the posted chan) + sys_create (the Acreate contract) + sys_fd2path"
    requirement: INT-01
    verification:
      - kind: unit
        ref: "cargo test -p port — port/tests/devdup_devsrv_test.rs 2/2 (devdup_walk_validation; devdup_devsrv_device_flow — resolve-by-number, mode mismatch, absent-fd errors, registry resolution of #d/#s, create/write/listing, Eexist/already-posted/OTRUNC refusals, open-returns-posted, and the T-05-11 clone-outlives-the-registrant pin)"
        status: pass
      - kind: integration
        ref: "builds virtshell/virtexec/default/raspi4b + clippy virtexec clean + boot-test fan (virtexec/virtmulti/virt/virtpanic) all PASS; the plan's grep legs land (pub mods, sys_create, registry rune constants)"
        status: pass
    human_judgment: false
  - id: D4
    description: "GATE: connectembed completes — pipe created, the child dups both ends onto 0/1 and execs /boot/paqfs, paqfs reads+verifies /rootfs.img (label + fingerprint print) and parks its io server on the pipe read (serving), then boot stops at the loud FVERSION wall (syscall 40) — never a deadlock, never a panic"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: ".planning/phases/05-shell-boot-one-command-integration/05-06-boot-r9.log: 'paqfs...' + 'sched: switch 1->2' + 'root: Tue Nov  3 01:05:45 GMT 2026' + 'fingerprint: ac4b6bd8…' + 'process 2 exited' + 'version...syscall 40 unimplemented' + 'boot: can't init 9P: syscall not implemented' + 'process 1 exited'; zero 'panic|Unhandled interrupt', zero 'can't exec paqfs', zero 'fatal: pipe' (the plan's grep chain: all pass)"
        status: pass
      - kind: integration
        ref: "boot-test all five profiles PASS post-gate (virtexec/virtmulti/virtnotes/virt/virtpanic)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The gate's two trace-settled gaps closed per the plan's carried discipline: sys_seek (SEEK 39, measured ×72 — paqfs's bio Bseek; the gate's first run named it) and the cold-syscall-buffer residency fix (the gate's EL1h DataAbort at FAR 0x36000 on paqfs's emalloc'd brk_-grown buffer)"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "the first gate run's failure evidence ('syscall 39 unimplemented' + the EL1h TrapFrame dump at FAR 0x36000) vs the re-run's green (zero 'syscall 39' lines — getenv's seeks resolve; paqfs's Bseek drives the whole-image verify read; the abort is gone)"
        status: pass
    human_judgment: false
  - id: D6
    description: "devsrv/devdup/sys_create have no EL0 caller before the FVERSION wall (boot.c:141 fversion precedes :144 srvcreate — the plan's gate prose inverted that order): the device semantics are host-pinned end-to-end (D3), but live boot-chain exercise arrives with 05-07's fversion/mount"
    verification: []
    human_judgment: true
    rationale: "Structural sequencing, not a gap: the binding gate greps all pass and every device op is host-tested over the real hook shape; no automated evidence can exercise #s/#d in the boot chain until devmnt lands (05-07) — the 05-05 D4 precedent for unwired-until-later surfaces"

# Metrics
duration: 110 min (incl. the kstack-overflow debug bisect + two 300s gate runs + fan re-runs)
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 06: devpipe/devdup/devsrv + the connectembed gate (paqfs serving, FVERSION wall) Summary

**The pipe wall is demolished: r9's devpipe carries Harvey's real boot(8) through connectembed — pipe, fork, dup onto 0/1, exec /boot/paqfs, which reads and verifies the whole /rootfs.img and parks its 9P io server on the pipe read (serving) — and boot stops exactly at the loud FVERSION(40) wall, panic-free. The plan's TDD core delivered the Arc-shared two-queue pipe (Pitfall 4's anti-deadlock property), the real rfork fd-group semantics (the phase's third plan-text flag inversion — fork(2) governs: NEITHER share / RFFDG copy / RFCFDG empty), devdup's #d/N files and devsrv's #s registry, sys_pipe/sys_dup/sys_create/sys_fd2path, and the MAX_PROCS/MAX_FDS resize. The gate then surfaced two trace-settled gaps the plan's discipline exists for — SEEK(39) (paqfs's bio Bseek) and the cold-syscall-buffer class (an EL1h abort on paqfs's brk_-grown heap buffer, closed by generalizing 04-08's explicit residency) — both implemented, never silenced. The debug of record: an inline [Option<Chan>; 32] in Proc::new overflowed the 16 KiB kstack into the proc's root-table page (stack reads returned UART-register bytes mid-print); the FdGroup is heap-backed now.**

## Performance

- **Duration:** 110 min
- **Started:** 2026-11-03T06:37:16Z
- **Completed:** 2026-11-03T08:27:49Z
- **Tasks:** 3
- **Files modified:** 19 (10 r9 source modified + 4 r9 devices/tests created... counting: r9 created 8 (4 source + 3 test + 1 evidence), r9 modified 9, harvey 3 (.planning artifacts incl. the evidence log))

## Accomplishments

- **Task 1 (TDD): the pipe substrate is real and deadlock-proof by construction.** RED: 6 devpipe + 3 fdgroup host tests against compiling skeletons, evidence verified `RED_EVIDENCE_OK` (target `pipe_pair_write_read_round_trip` failed on the planned write-read assertion; 8/9 failing) at `port/tests/05-06-red-evidence.json` (r9 `1a9cf09`). GREEN (r9 `9785410`): devpipe (the Arc<Lock<Pipe>> both ends reference — two cross-connected 4 KiB queues per the 9k semantic reference; blocking reads via the injected block/wake hooks; EOF-after-drain; named overflow; qreopen), dat.rs's `pipe` Arc slot + `copen` (the donor's COPEN) + the dup reference rule, namec's '|' registry arm + DEV_PIPE legs + `close_chan`, and proc.rs's fd group (`Arc<Lock<FdGroup<MAX_FDS>>>`, the checked-out-chan op pattern, rfork's share/copy/empty arms per fork(2), MAX_PROCS=16/MAX_FDS=32 with named exhaustion, the pipe hooks). sys_pipe/sys_dup land validation-first (validated user array, pair allocation before any install, evict-after-install per the donor's `date > #d/1` note). RFNOTEG/RFCFDG join rfork's supported set (RFNOTEG a no-op — per-pid notes have no group to split; paqfs's rfork requires it).
- **Task 2: the small devices the boot chain fatals on** (r9 `2046684`): devdup (registry 'd') — `#d/N` opens resolve the CALLING proc's fd N through the injected hook and return ITS chan (the donor dupopen; rcmain's `. -i '#d/0'` contract); devsrv (registry 's') — create (OWRITE-only, Eexist-on-dup) + fd-number write binding an Arc-safe reference clone (T-05-11: host-pinned that the clone outlives the registrant's close) + listing + open-returns-the-posted-chan (donor srvopen); sys_create (the donor chan.c Acreate: parent walk, existing-file→OTRUNC-open fallback per OEXCL, device create dispatch — devenv/devsrv are the creators) + sys_fd2path (traced ×3). Host tests pin the whole acceptance surface (the gate can't reach it — see D6).
- **Task 3 (GATE): connectembed completes end-to-end.** The first gate run surfaced exactly two gaps and the iterate-discipline closed both (r9 `9b5526c`): (1) `syscall 39 unimplemented` — SEEK, measured ×72 in the 05-03 trace, paqfs's libbio Bseek rides it (sys_seek: base 0/1 per the donor, Eisdir, Enegoff, SEEK_END loud-unsupported); (2) an EL1h DataAbort at FAR 0x36000 — paqfs's `emalloc`'d readBlocks buffer is brk_-grown (demand-paged) and the kernel's pread made its first touch (the phase-04 deferred #3 class, post-exec half) — closed by `fault::ensure_user_resident`, generalizing 04-08's note-delivery residency to any writable user buffer before kernel deref (T-05-06 holds: explicit page-in, never a nested fault). The re-run (committed `05-06-boot-r9.log`): `paqfs...` → the child execs → `root: Tue Nov  3 01:05:45 GMT 2026` + `fingerprint: ac4b6bd8…` (paqfs read+verified the WHOLE image through devroot's raw node) → the io server parks on the pipe read → `process 2 exited` clean → `version...` → `syscall 40 unimplemented` (the wall) → boot's fatal → clean exit. Zero `panic`/`Unhandled interrupt`, zero `can't exec paqfs`, zero `fatal: pipe`, zero `syscall 39` remnants.
- **The kernel-stack-overflow debug (the plan's debug trail of record):** the staged fan regressed at init's first rfork with a corrupting print ('d'+13×'u'); bisect proved the fd-share arm innocent and a stack probe found SP at `0xffff90000004200` with stack-resident reads returning UART-register bytes — the inline `[Option<Chan>; 32]` (~5.7 KB) in Proc::new, inside the rfork→new_user_proc chain on the 16 KiB kstack, overflowed into the proc's root-table page below it. The FdGroup's slots are a heap-backed Vec now (no giant stack temporary ever materializes); KSTACK_PAGES stays 4 (the 05-01/04-04 discipline); the full fan proves the fix.
- **All staged profiles green through the new machinery** — twice (the fix and the final tree): virtexec/virtmulti/virtnotes/virt/virtpanic all PASS, with init's rfork children now SHARING its fd table per fork(2) (the smokefork fd-ops and the note path ride it).

## Task Commits

Each task was committed atomically (two-repo convention — r9 commits land on `main` of `requaos/r9`):

1. **Task 1 RED: failing devpipe + fd-group tests + evidence record** — r9 `1a9cf09` (test)
2. **Task 1 GREEN: devpipe + fdgrp + fd-group rfork semantics + sys_pipe/sys_dup + sizing + hooks** — r9 `9785410` (feat)
3. **Task 2: devdup + devsrv + sys_create/sys_fd2path + host tests** — r9 `2046684` (feat)
4. **Task 3: GATE — sys_seek + the cold-buffer residency fix, gate green** — r9 `9b5526c` (feat)
5. **HowItWorks.md material-change section (the header's mandate)** — r9 `5af3fe1` (docs)

No REFACTOR commit — the GREEN code was fresh-written and clippy-clean; nothing needed a cleanup pass (tdd.md: commit only on change). TDD gate sequence holds: RED (test) precedes GREEN (feat), both present.

**Plan metadata:** (the harvey docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS + the evidence log + deferred-items + WINDOWS.md)

## Files Created/Modified

- `../r9/port/src/devpipe.rs` — the pipe device: Pipe (two cross-connected bounded queues + qref open-end counts), the block/wake hooks, attach/walk/open/read/write/close/stat
- `../r9/port/src/fdgrp.rs` — FdGroup<N>: the donor's Fgrp as a shareable group object (heap-backed slots; take/store/ref_clone/alloc/pair/deep_copy/list_open)
- `../r9/port/src/devdup.rs` — per-fd files `#d/N` via the injected resolver (registry rune 'd')
- `../r9/port/src/devsrv.rs` — the `#s` service registry (create/write/listing/open-returns-posted; rune 's')
- `../r9/port/src/dat.rs` — DEV_PIPE/DEV_DUP/DEV_SRV indices; Chan's `pipe` Arc slot + `copen` + mark_open + the dup pipe-reference rule
- `../r9/port/src/namec.rs` — '|'/'d'/'s' registry arms, the DEV_PIPE/DUP/SRV walk/open/stat legs, `close_chan` (the devclose dispatch), `create_chan` (the Acreate device half), `walk_one`
- `../r9/port/src/pgrp.rs` — `chain_at` goes pub (sys_create's dot redirect)
- `../r9/port/src/lib.rs` — the three new modules
- `../r9/aarch64/src/syscall.rs` — sys_pipe/sys_dup/sys_create/sys_fd2path/sys_seek; the fd handlers ride the checkout pattern with DEV_PIPE/DUP/SRV legs; RFNOTEG/RFCFDG in SUPPORTED; residency wired into the buffer paths
- `../r9/aarch64/src/proc.rs` — the fd group field + fd op API (take/store/ref_clone/install paths/pair/path), rfork fd-group arms, MAX_PROCS/MAX_FDS resize, pipe block/wake + fd resolver/listing hooks
- `../r9/aarch64/src/main.rs` — the three devices' hook registrations before the first exec
- `../r9/aarch64/src/fault.rs` — `ensure_user_resident` (the cold-buffer class's closure)
- `../r9/HowItWorks.md` — the 05-06 section
- `../r9/port/tests/{devpipe_test.rs,fdgroup_test.rs,devdup_devsrv_test.rs}` + `05-06-red-evidence.json` — the suites + RED evidence
- `.planning/phases/05-shell-boot-one-command-integration/05-06-boot-r9.log` — the committed gate evidence
- `.planning/phases/05-shell-boot-one-command-integration/deferred-items.md` — the SEEK item resolved; the exit-sweep deferral recorded
- `.planning/WINDOWS.md` — entry 14 (the CREATE stub) fixed; two new deviation entries (the fd-group inversion; the cold-buffer residency)

## Decisions Made

- **The donor's two-queue pipe governs** over the plan's "single shared queue" prose (the shared Pipe OBJECT is what both ends reference; paqfs's io(1) reads AND writes fd 1, which a single FIFO cannot serve).
- **fork(2) governs the fd-group flags** (the third inversion; the bullet's RFFDG/default are swapped relative to sysrfork + libc fork()'s connectembed requirement).
- **Heap-backed fd slots** (the kstack-overflow lesson — no giant stack temporaries in proc construction).
- **RFNOTEG as a no-op flag** (per-pid notes have no group channel; paqfs's rfork requires it).
- **devsrv/devdup's live boot-chain exercise is 05-07's** (boot.c's fversion precedes srvcreate; the plan's gate prose had the order inverted — the binding greps pass and the semantics are host-pinned).
- **Exit-time fd sweep deferred** (not gate-load-bearing; the donor's closefgrp lands when a measured caller needs it — deferred-items.md).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Plan text bug] the fd-group bullets invert RFFDG vs the default vs fork(2)**
- **Found during:** Task 1 RED test design (cross-checking the bullet against sys/man/2/fork + 9k sysproc.c — the 05-04/05-05 precedents made the check routine)
- **Issue:** the truth bullet + Task-1 behavior + action text all say "default copies, RFFDG shares". fork(2): "RFFDG: the invoker's file descriptor table is COPIED; otherwise the two processes SHARE". sysrfork: RFFDG→dupfgrp (copy), RFCFDG→dupfgrp(nil) (empty), neither→incref (share). libc fork()=rfork(RFPROC|RFFDG|RFREND)=COPY — connectembed's child dup/close is only safe under the donor reading.
- **Fix:** implemented + tested the donor semantics (share/copy/empty + the rfork wiring); recorded in the test header, the proc.rs/syscall.rs comments, HowItWorks.md, and WINDOWS.md entry 16.
- **Files modified:** `../r9/aarch64/src/proc.rs`, `../r9/aarch64/src/syscall.rs`, `../r9/port/tests/fdgroup_test.rs` (header note)
- **Verification:** `fd_group_matrix_share_copy_empty` + `fd_group_copy_shares_the_pipe_queue` pin the matrix; the gate exercises fork() (COPY) and paqfs's rfork(RFFDG) (COPY) live; the staged fan exercises SHARE (init's rfork(RFPROC|RFMEM) children)
- **Committed in:** `9785410`

**2. [Rule 3 - Blocking] the plan's verify/gate build order would boot the wrong kernel (the 05-04 lesson's third instance)**
- **Found during:** Task 3 prep — the plan's Task-2 verify fan ends with virtpanic and Task 3's gate uses `$(ls target/.../debug/aarch64)`; all configs share that path, so the gate would boot the virtpanic kernel
- **Fix:** rebuilt virtshell LAST before the gate run (the established lesson: the gated config builds last)
- **Files modified:** none (ordering only)
- **Verification:** the gate booted the virtshell kernel (banner + the full boot chain)
- **Committed in:** n/a (ordering)

**3. [Rule 1 - Bug] the inline FdGroup array overflowed the 16 KiB kernel stack (the debug of record)**
- **Found during:** Task 1 verify — all three staged profiles regressed at init's first rfork: the next print corrupted mid-string ('d'+13×'u') and the kernel wedged
- **Issue:** `FdGroup<32>`'s inline `[Option<Chan>; 32]` (~5.7 KB) materialized on the stack inside Proc::new; the rfork→new_user_proc chain on the 16 KiB kstack overflowed into the proc's root-table page below — a stack probe showed SP at `0xffff90000004200` with stack-resident reads returning UART-register bytes (the smashed table mis-translated the kstack pages)
- **Fix:** heap-backed slots (`Vec<Option<Chan>>` filled in place — no giant stack temporary ever); bisect-proven (the fd-share arm was exonerated first; an 8-page kstack probe confirmed the overflow class, then the real fix kept KSTACK_PAGES=4)
- **Files modified:** `../r9/port/src/fdgrp.rs`
- **Verification:** full port suite + all three Task-1 fan profiles PASS with the share semantics active (twice: post-fix and final tree)
- **Committed in:** `9785410` (part of Task 1 GREEN)

**4. [Rule 1 - Bug] the gate's EL1h DataAbort — the cold-syscall-buffer class (phase-04 deferred #3's post-exec half)**
- **Found during:** Task 3's first gate run — paqfs died mid-init at FAR 0x36000: its `emalloc`'d readBlocks buffer is brk_-grown (demand-paged), and the kernel's pread made the first touch
- **Fix:** `fault::ensure_user_resident` — 04-08's explicit-residency pattern generalized to any writable user range (classified Bss/Stack zero page-in after validate_user_range, before any deref, no locks held — never a nested fault, T-05-06 holds); wired into pread/pwrite/errstr/await/fd2path/pipe's buffer paths
- **Files modified:** `../r9/aarch64/src/fault.rs`, `../r9/aarch64/src/syscall.rs`
- **Verification:** the re-run's log shows paqfs's whole-image verify read complete (label + fingerprint) with zero aborts
- **Committed in:** `9b5526c`

**5. [Rule 3 - Trace-settled gap] SEEK(39) — the gate named it, the trace measured it (×72), paqfs's bio Bseek needs it**
- **Found during:** Task 3's first gate run (`syscall 39 unimplemented` — the plan's carried discipline: "the loud error names the number; trace-settled gaps get implemented, never silenced")
- **Fix:** sys_seek per the donor (base 0 absolute / base 1 relative with negative deltas, Eisdir on directories, Enegoff below zero; SEEK_END loud-unsupported — no measured pre-wall caller); the wire return is the new offset (bio's Bseek checks it)
- **Files modified:** `../r9/aarch64/src/syscall.rs`
- **Verification:** the re-run's log has zero `syscall 39` lines (getenv's seeks resolve too — the phase's deferred SEEK item is marked resolved); paqfs's verify read completes
- **Committed in:** `9b5526c`

**6. [Rule 2 - Missing critical] RFNOTEG accepted in rfork's SUPPORTED set (caught by code-reading paqfs.c before the gate)**
- **Found during:** Task 1 design — paqfs's `rfork(RFFDG|RFPROC|RFNAMEG|RFNOTEG)` (paqfs.c:247) would otherwise fail "unsupported flags" and paqfs would never serve
- **Fix:** RFNOTEG accepted as a no-op (note delivery is per-pid — devproc's post_note has no group channel, so there is no note group to split); RFCFDG joined it (the fd-group arm)
- **Files modified:** `../r9/aarch64/src/syscall.rs`
- **Verification:** the gate shows paqfs's rfork succeed (process 2 exits clean after forking the io server)
- **Committed in:** `9785410`

**7. [Rule 3 - Plan text] the gate truth's srvcreate/fversion order is inverted vs boot.c (documentation-level)**
- **Found during:** Task 3 design — the plan says "srvcreate writes #s/boot — then fversion hits the wall"; boot.c:141 fversion precedes :144 srvcreate
- **Fix:** none required — the binding verify greps pass as written; devsrv's create/write/open semantics are host-test-pinned (D3) and the live srvcreate arrives in 05-07 (fversion/mount work). Recorded here + in coverage D6.
- **Files modified:** none
- **Committed in:** n/a

---

**Total deviations:** 7 auto-fixed (3 Rule 1 bugs — one plan-text, two code; 2 Rule 3 blocking/trace-settled; 1 Rule 2 missing-critical; 1 documentation-level)
**Impact on plan:** Every deviation is the TDD/gate shape doing its job — the third flag inversion held the donor-governs precedent cleanly, the kstack overflow was caught by the staged fan before the gate ever ran, and the two gate-surfaced gaps are exactly the plan's iterate-discipline operating as designed. No scope creep: nothing beyond the measured set + the gate-surfaced pair was implemented (SEEK_END stays loud; the exit sweep stays deferred).

## Issues Encountered

- **The kstack-overflow debug consumed the bulk of the plan's time** (the 'u'-corruption bisect) — recorded as deviation 3 with the full trail in HowItWorks.md; the lesson (no giant stack temporaries in proc construction) is now a pattern note.
- **qemu/fdtput are devshell-provided** — the gate command runs inside `nix develop` from ../r9 per the 05-04/05-05 documented pattern.
- **Gate runtime:** the end state is the paqfs io child parked on its pipe read (a server blocked on read is the correct state — 05-07's devmnt will drive requests); qemu's teardown is the harness's timeout per the established gate shape.

## User Setup Required

None - no external service configuration required. (Driving run-r9.sh by hand needs qemu-system-aarch64 + fdtput on PATH — the r9 devshell provides both; `nix develop` from ../r9, then the gate command from this repo root.)

## Known Stubs

- **devsrv's open-returns-posted-chan and devdup's resolve are host-pinned only this plan** — structural sequencing (the FVERSION wall precedes srvcreate in boot.c), not a stub; the live boot-chain exercise is 05-07's by design. (Not a WINDOWS entry — the semantics are complete and host-tested.)
- **SEEK_END is loud-unsupported** (`sys_seek` errors "SEEK_END unsupported this era") — no measured pre-wall caller; a measured caller later lands it. Intentional, documented in the handler.
- **The non-RFPROC fd-group split (RFFDG/RFCFDG without RFPROC) stays a loud -1** — no measured caller (the 05-04/05-05 discipline); the gate's forks are all RFPROC.
- **Exit-time fd-group sweep deferred** — deferred-items.md records the donor closefgrp shape for the plan that needs it (Moribund procs pin their fd groups this era; harmless — the kernel halts after all-exited, and connectembed's closes are all explicit).
- 05-04/05-05's known stubs are unchanged (cons reads -1 until 05-08; devproc dir/status/args until 05-08/05-09).

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: syscall_surface | ../r9/aarch64/src/fault.rs | ensure_user_resident pages in user buffers at syscall entry (explicit, no locks held, classified Bss/Stack, mapping-log recorded) — the T-05-06-safe closure of the T-04-11 caveat class; flagged for the phase audit's visibility (it widens what syscall handlers can safely touch, on purpose, with the validate-first discipline unchanged) |

(T-05-10 held: the Arc-shared queue with clone-on-dup + the host-pinned fork matrix + the bounded named-overflow queue. T-05-11 held: devsrv's write clones the registered chan — host-pinned that it outlives the registrant; the listing exposes names only. T-05-12 held: every new handler is validation-first (sargs + validate_user_range + 32-bit int masking; sys_pipe writes through the validated array only; sys_seek validates the fd and never touches user memory). T-05-SC: zero package installs.)

## Next Phase Readiness

- **05-07 (devmnt)**: the pipe substrate is proven with a live server parked on it; fversion's Tversion exchange writes/reads boot's end (devpipe's read/write with the server draining); mount(fd) resolves `#s/boot`'s posted clone via devsrv's open; the mount table + union walk it lands into are 05-04's. Boot's next wall after FVERSION is the mount path itself.
- **05-08**: init's setenv/cpenv ride sys_create→devenv (both landed); newns's rfork(RFENVG|RFCNAMEG) env split gets its live exercise past the wall; cons reads + the namespace-file stanza remain its scope.
- **05-09**: the smoke sweep rides the fd-group semantics (rc pipelines) and `#d/0` (rcmain's interactive path).
- r9 HEAD: `5af3fe1` (main). Harvey HEAD: `8e3546583` + the metadata commit.
- The staged-profile fan is green through the fd-group/pipe/small-device machinery (all five profiles); the phase's deferred SEEK item is resolved (sys_seek landed).

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: r9 port/src/{devpipe.rs,fdgrp.rs,devdup.rs,devsrv.rs}, port/tests/{devpipe_test.rs,fdgroup_test.rs,devdup_devsrv_test.rs,05-06-red-evidence.json}; harvey .planning/phases/05-shell-boot-one-command-integration/{05-06-SUMMARY.md,05-06-boot-r9.log,deferred-items.md} — all FOUND on disk and (r9 side) committed.
- Commits: r9 1a9cf09 (RED test), 9785410 (GREEN feat), 2046684 (Task 2 feat), 9b5526c (Task 3 gate feat), 5af3fe1 (docs) — all FOUND in the r9 git log; RED precedes GREEN (TDD gate sequence holds; no REFACTOR — the GREEN code was fresh-written and clippy-clean).
- Plan-level <verification>: (1) host tests — `cargo test -p port` 46/46 green (6 devpipe + 3 fdgroup + 2 devdup_devsrv new; the RED evidence verified RED_EVIDENCE_OK, target_test_failed, exit 101, 8/9 failing); (2) builds virtshell/virtexec/default/raspi4b + clippy virtexec clean; (3) the connectembed gate — 05-06-boot-r9.log shows paqfs... → the exec'd child → the whole-image verify print (label + fingerprint) → the io server parked on the pipe read → version... → syscall 40 wall → clean exits; the plan's grep chain all-pass; zero panic/Unhandled/can't-exec/fatal-pipe; (4) boot-test fan — all five profiles PASS on the final tree (twice for the fd-group fix); ureg-check ran in each boot-test leg (R9_HARVEY_DIR set).
- Acceptance criteria re-run at final state: Task 1 (Arc-shared pipe with block/wake + EOF, host-proven; fd-group matrix per fork(2); tables 16/32 with named exhaustion; staged profiles green) — PASS. Task 2 (devdup resolve-by-number via hook; devsrv create/write/listing with the Arc-safe clone; sys_create dispatches with validated paths; all green) — PASS. Task 3 (the gate log shows the complete connectembed round trip to the loud FVERSION wall; every negative scan empty; staged profiles green) — PASS.

