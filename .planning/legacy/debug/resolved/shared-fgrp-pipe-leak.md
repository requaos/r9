---
status: resolved
trigger: "r9 kernel defect from Harvey plan 20-07: a fd table shared through a plain rfork(RFPROC) child (no RFFDG) leaks a pipe write end past the last sharer's exit, so an rc backquote reader of `rfdprobe -s` hangs forever (aarch64, HVF and TCG). Controls fork() and no-pipe complete."
created: 2026-11-19T00:00:00Z
updated: 2026-11-19T03:10:00Z
---

## Current Focus

bug_class: Bohrbug (deterministic: hangs every run, HVF and TCG; no timing dependence — the leak is a missing Drop on a diverging path)
hypothesis: H1 — aarch64 `exit_tail` (syscall.rs:2093) is `-> !`; its local `detached` (the proc's fd-group Arc, swapped out for the empty singleton) is never dropped because the function never returns (park_forever / sched_from_moribund). A sharer that exits with count>1 therefore leaks one strong ref permanently, so the LAST sharer's exit still sees strong_count==2 and skips drain_all → the pipe writer in slot 1 is never close_chan'd → devpipe writer count never reaches 0 → rc's backquote read never sees EOF.
test: in an experiment clone of r9 57e2952, (a) instrument exit_tail to print pid + strong_count(&detached) and boot the 20-07 image with the probe driver typing `x=`{/bin/rfdprobe -s}; echo DIAGS $x`; (b) add `drop(detached)` before the diverging tail and rerun; (c) revert (b) and rerun
expecting: (a) unfixed: child exit count=2, parent exit count=2, hang (no DIAGS). (b) fixed: child count=2, parent count=1, `DIAGS rfdprobe: share ok`. (c) hang returns. H1 is FALSE if the unfixed parent exit already prints count=1 (then the leak is downstream of the drain — devpipe count bookkeeping), or if (b) still hangs.
candidate_causes:
  - code (H1): exit_tail divergence leaks the detached Arc (aarch64 only; x86_64 has drop(detached) since c4f2f2c)
  - code (H2): another fd_group Arc clone held elsewhere (wait record, devdup, devproc) — grep shows the only aarch64 clone is rfork's share arm (proc.rs:2546); still falsified by (a) if counts read 2/1
  - code (H3): devpipe writer open-count double-bumped on the share path — rfork share is an Arc clone with no Chan::dup, so no count bump; falsified by (b)
  - data/environment: rc's backquote pipe handling or accel — eliminated by controls (echo/forkstress backquotes complete; HVF and TCG identical)
and_gate: yes for TRIGGER, no for CAUSE — the hang needs (shared group, no RFFDG) AND (the leaked ref) AND (the group holds the last pipe writer with a reader awaiting EOF); fork() lacks the first, the no-pipe control lacks the third. One code defect (H1) is the cause.
status_note: H1 CONFIRMED by (a)+(b). Fix committed locally in r9 as 3a1c9e5 on main b82ee9d (unpushed; tree == tested b77f764, kernel ELF byte-identical). Guardrail accepted (see Resolution.verification). AWAITING human verification; then archive + KB entry.
next_action: on owner confirmation — move this file to .planning/debug/resolved/, append the KB entry (Prevention block); owner/coordinator decide D2 (bare RFCFDG singleton), the aarch64 borrow-close port, the r9 push and the Harvey pin advance + arm64 HVF series
reasoning_checkpoint:
  hypothesis: "aarch64 exit_tail (-> !) holds the detached fd-group Arc as a local and diverges, so the Arc is never dropped; a non-last sharer's exit therefore leaks one strong ref, the last sharer reads strong_count 2, skips drain_all, and the pipe writer in the shared table is never close_chan'd — rc's backquote reader never sees EOF."
  confirming_evidence:
    - "(a) instrumented unfixed kernel: child pid 7 strong=2, then parent pid 6 (last sharer) strong=2, hang (TIMEOUT at done)"
    - "(b) the same kernel + only `drop(detached)`: child strong=2, parent strong=1, `DIAGS rfdprobe: share ok`"
    - "x86_64 exit_tail carries the explicit drop since c4f2f2c (#44) with the comment 'every local still live at the tail rides the dead kstack forever'; aarch64 never received it"
  falsification_test: "the unfixed last sharer printing strong=1 (leak downstream in devpipe), or (b) still hanging — neither happened"
  fix_rationale: "the sweep moves into a returning helper (proc::release_fd_group), so the detached reference is released at its return, before exit_tail diverges — the reference-release itself is fixed, not the EOF symptom; the drain/close behaviour is unchanged"
  blind_spots: "other locals in frames that diverge into exit_tail (exec's post-free kill path, sys_exits' msg Vec) still never drop — heap-only for the cases read (no Chan/Arc group handle found), not measured; the 19-10 borrow-close (devmnt duplicate fids) and the #44 husk-payload sweep are also unported to aarch64 (separate defects)"
  candidate_causes:
    - "code: exit_tail divergence leaks the detached Arc (CONFIRMED)"
    - "code: another holder of the group (ELIMINATED by (b))"
    - "code: devpipe open-count double bump (ELIMINATED by (a): the drain never ran)"
    - "environment/data: accel or rc backquote handling (ELIMINATED by the controls and HVF=TCG)"
  and_gate: "trigger needs shared table AND leaked ref AND pipe writer + EOF reader; the cause is the single code defect"
coordinator_constraint: plan 20-08 will commit an in-place RFFDG/RFCFDG change (sys_rfork non-RFPROC branch) to the MAIN r9 checkout's aarch64/src/syscall.rs. The final fix commit must be based on the main checkout's HEAD at commit time, and rfdprobe -s must be re-shown to reproduce (hang) on THAT base before the fix and complete after it.
fix_design: B — move the exit sweep's fd-group half into a RETURNING helper `proc::release_fd_group(slot, empty, close)` in aarch64/src/proc.rs (every local, the detached Arc above all, drops at its return, before exit_tail diverges); exit_tail calls it. The drain/close semantics stay byte-for-byte (borrowed close over the drained Vec — the 19-10 consume-one-at-a-time change is a separate devmnt-clunk defect, NOT ported here). Host regression tests with real devpipe pipes in proc.rs.
scratch_paths: clone = scratchpad/r9-fgrp; probe driver = scratchpad/probe/{probe.sh,p20-07-probe.expect,bootfs.paq}; scratchpad = /private/tmp/claude-501/-Users-neilskinner-code-harvey/55ded085-3397-4d2c-931d-1d3a7a81443a/scratchpad

## Symptoms

expected: after rfdprobe -s and its shared-fd-table RFPROC child both exit, the pipe write end held in the shared fd table is closed, the backquote reader sees EOF, rc prints `DIAGS ...`
actual: `x=`{/bin/rfdprobe -s}; echo DIAGS $x` hangs forever; all procs have exited; no DIAGS output
errors: none (hang)
reproduction: boot the 20-07 image (rfdprobe baked in) on r9 57e2952 under qemu virt aarch64 (HVF or TCG), type the backquote line at the % prompt
started: first observed by 20-07 on r9 57e2952; unknown whether ever worked (rfork share path)
controls: `x=`{/bin/forkstress -n 1}` (fork(), RFFDG copy) completes; `/bin/rfdprobe -s; echo DIAGT $status` (no pipe) completes; `x=`{/bin/echo hi}` completes

## Eliminated

- hypothesis: H3 — devpipe's writer open count is double-bumped on the share path (the drain runs but the count never reaches 0)
  evidence: experiment (a) — the last sharer read strong_count 2, so drain_all / close_chan never ran at all; and the rfork share arm is a bare Arc clone with no Chan::dup
  timestamp: 2026-11-19T00:40:00Z

- hypothesis: H2 — some other kernel path (wait record, devdup, devproc) holds an extra clone of the shared group
  evidence: experiment (b) — adding only `drop(detached)` in exit_tail made the last sharer read strong_count 1; no other holder exists (grep: rfork's share arm is the only aarch64 fd_group clone site)
  timestamp: 2026-11-19T00:55:00Z

- hypothesis: environment/data — accelerator-specific behaviour or rc's backquote pipe handling
  evidence: HVF and TCG behave identically (20-07 and the matrix); the echo / forkstress backquote controls complete; the fixed kernel completes the same rc line with the same image
  timestamp: 2026-11-19T02:50:00Z

## Evidence

- timestamp: 2026-11-19T00:00:00Z
  checked: .planning/debug/knowledge-base.md (2 entries)
  found: no keyword match (fd table / pipe / rfork / hang not in either entry). x64-kzero entry carries a useful note — run `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` explicitly; `cargo xtask test` in nix develop skips the arch leg
  implication: no known-pattern candidate; investigate fresh

- timestamp: 2026-11-19T00:10:00Z
  checked: build/aarch64/rfdprobe.c -s path
  found: -s does rfork(RFPROC) (no RFFDG/RFCFDG) → child exits(nil) at once → parent wait()s, prints, exits(nil). Parent's fd 1 is rc's backquote pipe write end
  implication: the only pipe-writer holders are parent P and child C, both via ONE shared FdGroup Arc

- timestamp: 2026-11-19T00:12:00Z
  checked: r9 aarch64/src/proc.rs:2540-2547 (rfork fd-group arm) and aarch64/src/syscall.rs:2093-2146 (exit_tail)
  found: neither-flag rfork = parent.fd_group.clone() (Arc count 2). exit_tail does `let detached = mem::replace(&mut proc.fd_group, empty_fdgroup())`, drains only if strong_count(&detached)==1, comment says "count > 1 ... the detached Arc just drops" — but exit_tail is `-> !` and ends in park_forever()/sched_from_moribund(), so `detached` is NEVER dropped (its Arc pointer rides the freed kstack). No other fd_group.clone() site exists in aarch64.
  implication: C's exit leaks one strong ref; P's later exit sees count 2, skips the drain; the pipe writer is never close_chan'd → reader never sees EOF. Explains the -s hang. fork() control: C's group is a private copy (count 1) → drained; P count 1 → drained → passes. No-pipe control: nothing waits for EOF → passes.

- timestamp: 2026-11-19T00:15:00Z
  checked: r9 x86_64/src/syscall.rs:1934-2009 and git log
  found: x86_64 exit_tail has an explicit `drop(detached)` before the diverging tail, added by c4f2f2c "fix(17-02): ... (WINDOWS #44)" (framed as a ~8 KiB-per-exit HEAP leak: "exit_tail DIVERGES ... so its locals never drop"); plus ab0a977 (19-10 cont 6) changed the drain loop from `for c in &drained` (borrow) to consume-one-at-a-time. Both commits touched only x86_64/ + port/ — never ported to aarch64.
  implication: x86_64 is already immune to the shared-group pipe leak (as a side effect of the #44 heap fix); the defect is aarch64-only and the fix is an arch edit (aarch64/src/syscall.rs), not port/src

- timestamp: 2026-11-19T00:40:00Z
  checked: experiment (a) — scratchpad clone r9-fgrp at 57e2952 + ONE println in exit_tail (`fgrpdbg: pid N detached strong=K`), kernel /nix/store/qhjyrdd1b6pcv8hdjspp5zflql6zvjzw-r9-kernel-virt-wip; image = snapshot of build/aarch64/bootfs.paq sha256 30f61249…99aa (20-07's brkprobe image, rfdprobe baked); TCG cortex-a72 virt,gic-version=3; typed `x=`{/bin/rfdprobe -s}; echo DIAGS $x`; driver = scratchpad/probe/probe.sh + copied .build/p20-07-probe.expect
  found: boot procs pid 2, 5 exit strong=1. After the typed line: `fgrpdbg: pid 7 detached strong=2` / `process 7 exited: ""` (the shared child), then `fgrpdbg: pid 6 detached strong=2` / `process 6 exited: ""` (rfdprobe itself — the LAST sharer, must read 1). Then nothing: `probe: TIMEOUT at stage: done` (240 s), no DIAGS line. Captures: scratchpad/probe/a-unfixed-tcg.{log,out}
  implication: the reproduction holds on my clone; a strong ref outlives the child (H1 or H2), and the last sharer's drain is skipped. H3 (devpipe double count) ELIMINATED — the drain never ran, so devpipe close bookkeeping was never reached. (b) discriminates H1 vs H2.

- timestamp: 2026-11-19T00:55:00Z
  checked: experiment (b) — same clone + println, plus ONE line `drop(detached);` after the drain block (before rendez_remove); kernel /nix/store/bpkvwm2p2k8dxs4ybs9b2bks8qaz875w-r9-kernel-virt-wip; same image/driver/TCG
  found: `fgrpdbg: pid 7 detached strong=2` (child) → `fgrpdbg: pid 6 detached strong=1` (parent, last sharer — drain runs) → `DIAGS rfdprobe: share ok` → `process 8 exited` (echo) → prompt. rc=0, `probe: DONE`. Captures: scratchpad/probe/b-fixed-tcg.{log,out}
  implication: H1 CONFIRMED (dropping the detached Arc alone restores the count to 1 at the last sharer and the hang is gone). H2 ELIMINATED (no other holder of the group: if one existed the parent would still read 2).

- timestamp: 2026-11-19T00:56:00Z
  checked: aarch64 host test leg on darwin — `nix develop -c cargo test -p aarch64 --bins --target aarch64-apple-darwin` in the clone
  found: 18 passed / 0 failed (baseline); runs in ~9 s
  implication: a host regression test in aarch64/src/proc.rs is feasible (port's devpipe works on the host: fdgroup_test.rs builds real pipes; an empty pipe read returns Ok(0) only after hangup)

- timestamp: 2026-11-19T01:20:00Z
  checked: fix B in the clone at 57e2952 — `proc::release_fd_group` + 3 host tests (proc::fdgrp_release_tests); `cargo test -p aarch64 --bins --target aarch64-apple-darwin`
  found: 21 passed / 0 failed (18 baseline + 3 new). One NEW warning vs baseline: `unused import alloc::sync::Arc` in syscall.rs (exit_tail was its only user at 57e2952); FdGroup/Lock/MAX_FDS unused imports are PRE-EXISTING at 57e2952
  implication: on a base where Arc is still used (20-08's b82ee9d uses Arc::new(Lock::new(..)) in sys_rfork) the warning set is unchanged

- timestamp: 2026-11-19T01:25:00Z
  checked: main r9 checkout — 20-08 landed b82ee9d `feat(20-08): aarch64 rfork fd-group split, devmnt write, DMDIR + union creates` on top of 57e2952 (aarch64/src/syscall.rs only)
  found: the bare (non-RFPROC) arm now does RFFDG = `proc.fd_group = Arc::new(Lock::new("fdgrp", deep_copy))` and RFCFDG = `proc.fd_group = proc::empty_fdgroup()` — the same shape as x86_64 syscall.rs:1569-1575. exit_tail is untouched by b82ee9d.
  implication: (1) the fix must rebase onto b82ee9d; (2) the old shared Arc is dropped normally by the bare rfork (a returning function), so rfdprobe's DEFAULT mode (child shares → bare RFFDG/RFCFDG → exit) no longer leaks P's ref even without the fix — only -s (shared child that exits directly) still reproduces. Must be re-measured on b82ee9d.

- timestamp: 2026-11-19T01:30:00Z
  checked: Fgrp audit, both arches (reading, NOT measured) — the bare RFCFDG arm on x86_64 (syscall.rs:1573-1574, pre-existing) and on aarch64 (b82ee9d) assigns `proc::empty_fdgroup()`, the GLOBAL singleton every Moribund husk holds (x86_64 proc.rs:2192, aarch64 proc.rs:1891), not a fresh table
  found: a live proc that does bare rfork(RFCFDG) and then opens fds installs them in the shared singleton: a later RFCFDG proc starts with those fds visible (not empty — wrong semantics), every husk "holds" them, and because the singleton's strong count is always >1 the exit sweep never drains them (same class: never-closed pipe writers / never-Tclunked fids). Harvey callers: sys/src/cmd/usb/usbd/dev.c:160 (rfork(RFCFDG) then opens /dev/null + /dev/cons x2, then exec), sys/src/libthread/main.c:188 (_schedexecwait), sys/src/cmd/rc/plan9.c:81 (rc `rfork F`)
  implication: SEPARATE defect D2 (not the 20-07 symptom; not exercised by rfdprobe, whose rfcfdg child opens nothing). Proposed fix: bare RFCFDG = `Arc::new(Lock::new("fdgrp", FdGroup::new()))` on both arches (the RFPROC|RFCFDG arm in proc.rs already does exactly that). NOT applied here — changes x86_64 kernel bytes and 20-08's fresh code; reported for owner/coordinator decision.

- timestamp: 2026-11-19T02:00:00Z
  checked: fix rebased onto b82ee9d (clone commit b77f764, clean rebase); manual mutation seeding at release_fd_group (scratchpad/mutants.sh — no cargo-mutants in the devShell), each mutant run against `-- fdgrp_release_tests`
  found: 5/5 KILLED — M1 leak-forget (core::mem::forget(detached) = the original defect's semantics; killed by "the child's exit released its reference" left 2 right 1, and the 3-sharer count 3 vs 2), M2 `>= 1`, M3 `== 2`, M4 no close dispatch, M5 no detach (slot.clone())
  implication: the tests assert the root-cause behaviour (the non-last sharer's reference release), not just the EOF symptom

- timestamp: 2026-11-19T02:15:00Z
  checked: r9 battery on b77f764 vs baseline b82ee9d (worktree scratchpad/r9-base), darwin, nix develop
  found: `cargo xtask check` rc=0; `cargo xtask clippy` (default x86-64) rc=0; `cargo xtask test` rc=0 (port 232 passed / 0 failed; arch leg skipped — no rustup in the devShell); `cargo xtask clippy --arch aarch64` rc=1 at BOTH fix and base with an IDENTICAL diagnostic set (4 pre-existing `unsafe block missing a safety comment` errors + the same warnings — no new warning; the Arc import stays used by b82ee9d's sys_rfork); explicit `cargo test -p aarch64 --bins --target aarch64-apple-darwin` 21/21 (18 + 3 new) — but 1 intermittent failure of `proc::sem_tests::wake_consumes_picked_registrations_in_order` seen
  implication: battery green except a flake that needed separating (next entry)

- timestamp: 2026-11-19T02:25:00Z
  checked: the sem_tests flake — 500 runs of each test binary (base b82ee9d: 18 tests; fix: 21 tests)
  found: base 1/500 failed, fix 1/500 failed, the SAME test both times. Failure `left: [21] right: [22]`. Mechanism (reading): sem_deregister_waiter uses Vec::swap_remove on the GLOBAL SEM_WAITERS; register_is_idempotent (pa 0x9100_0000) deregistering concurrently moves the Vec's last entry into its slot, reordering pa 0x9200_0000's [21,22] to [22,21], so sem_wake consumes 22 first
  implication: PRE-EXISTING test-isolation flake (parallel tests share a static), independent of this fix — reported, not fixed (also means production deregister can reorder other words' registration order; harmless to correctness because a woken waiter re-registers, but it is not strict FIFO)

- timestamp: 2026-11-19T02:50:00Z
  checked: revert-and-reconfirm matrix ON THE 20-08 BASE (scratchpad/probe/matrix.sh): unfixed = /nix/store/cqiqfc81zxz5c5v91kb37s9b4adqc7vg-r9-kernel-virt-b82ee9d, fixed = /nix/store/r35x9hawi27j0saqaz0am571b14mmmfl-r9-kernel-virt-b77f764; image snapshot sha256 30f61249…; modes s = `x=`{/bin/rfdprobe -s}; echo DIAGS $x`, d = `x=`{/bin/rfdprobe}; echo DIAGD $x`; TCG cortex-a72 + HVF -cpu host
  found: unfixed tcg s HANG (procs 7 and 6 exited, no DIAGS; TIMEOUT at done) / unfixed tcg d `DIAGD rfdprobe: ok` / unfixed hvf s HANG (same shape) / unfixed hvf d `DIAGD rfdprobe: ok` / fixed tcg s `DIAGS rfdprobe: share ok` / fixed tcg d `DIAGD rfdprobe: ok` / fixed hvf s `DIAGS rfdprobe: share ok` / fixed hvf d `DIAGD rfdprobe: ok`. Captures: scratchpad/probe/matrix/*.{log,out}
  implication: on b82ee9d the bug still reproduces through -s on both accelerators and is gone with the fix (revert-and-reconfirm PASS). The default mode no longer hangs even unfixed — b82ee9d's bare rfork(RFFDG/RFCFDG) replaces the child's shared handle inside a RETURNING function, so the shared ref is dropped there before the child exits; only a shared child that exits directly (-s, libthread proccreate procs) still hit the leak.

- timestamp: 2026-11-19T02:55:00Z
  checked: composed aarch64 gates, TCG, from a scratch Harvey clone at 8bad466f1 (scratchpad/harvey-gate; never the main checkout — the 20-08 executor was running smoke-r9-net there), `R9_SKIP_BUILD=1 R9_ACCEL=tcg nix run .#<app> --no-write-lock-file --override-input r9 git+file://<scratchpad>/r9-fgrp` (kernel r9-kernel-virt-b77f764). Userland = byte-verified copy (diff -r identical) of the main checkout's arm64/bin, arm64/lib, build/aarch64/out (the executor's 11:15 build) + .build/bin/mkpaqfs; bake/DTB/expect steps ran as defined; baked bootfs.paq sha256 2f8492f0…86bf
  found: boot-r9-smoke rc=0 `smoke-r9: ALL STAGES PASS`; boot-r9-net-smoke rc=0 `smoke-r9-net: ALL STAGES PASS`; boot-r9-tier1 rc=0 `tier1-r9: ALL STAGES PASS`, 39 T1-OK (incl. T1-OK writeprobe, T1-OK forkstress) + 23 T1-SKIP, 0 T1-FAIL (expected 39 + 23). Captures: scratchpad/gates-fix/
  implication: no regression in the composed gates. Deviation: step 1 (the lima userland build) skipped — kernel-only change, and the lima ~/harvey-build venue belongs to the concurrent executor

- timestamp: 2026-11-19T02:58:00Z
  checked: x86-64 kernel byte identity — `nix develop -c cargo xtask build --arch x86-64` (darwin) in r9-base (b82ee9d) and r9-fgrp (b77f764)
  found: both target/x86_64-unknown-none-elf/debug/x86_64 = 23283560 bytes, sha256 817a422b305f0bc78d0220476548e02ce1ce608a6c311d6f6c9ce54bf8894509 (identical)
  implication: the fix touches aarch64/src only; x86_64 kernel bytes unchanged (measured for the default config; by construction for kernel-x64/q35shell — no port/ or x86_64/ source changed)

## Resolution

root_cause: r9 aarch64/src/syscall.rs `exit_tail` (`-> !`, diverges into park_forever / sched_from_moribund) held the detached fd-group Arc (`let detached = mem::replace(&mut proc.fd_group, empty_fdgroup())`) as a local that is never dropped. Each non-last sharer's exit of a table shared via plain rfork(RFPROC) leaked one strong reference, so the last sharer read strong_count 2, skipped drain_all, and every chan in the table (here rc's backquote pipe write end at fd 1) stayed open forever — the reader never saw EOF. x86_64 has carried the explicit `drop(detached)` since c4f2f2c (17-02 #44, framed then as a heap leak); it was never ported to aarch64. aarch64-only.
fix: r9 3a1c9e5 (local, unpushed, on main b82ee9d) `fix(aarch64): release the exiting proc's fd-group reference before exit_tail diverges` — the sweep moves into a RETURNING helper `proc::release_fd_group(slot, empty, close)` (aarch64/src/proc.rs) so the detached reference drops at its return; exit_tail calls it with `port::namec::close_chan`. Drain/close semantics unchanged (borrowed close over the drained Vec). + 3 host tests `proc::fdgrp_release_tests`.
oracle_type: specified — the fork(2)/donor contract "a non-last sharer's exit gives up exactly its reference; the last sharer's exit closes every fd in the table" (asserted as strong_count after each release + close count) plus the end-to-end symptom oracle "the pipe reader sees EOF (read == Ok(0)) after the last sharer's exit". Minimized seed: two sharers, one pipe writer at fd 1 (the rfdprobe -s shape). Boundary neighbours: one holder (count 1 — fork()'s RFFDG copy) and three holders (count 3 → 2 → 1).
verification:
  target_test: { result: pass, detail: "proc::fdgrp_release_tests 3/3; cargo test -p aarch64 --bins --target aarch64-apple-darwin 21/21 (18 + 3)" }
  mutation_check: { result: pass, reason_if_skipped: "no cargo-mutants in the devShell — degraded to MANUAL seeding (scratchpad/mutants.sh) at release_fd_group", mutant_killed: "5/5: forget(detached) [the original defect], `>= 1`, `== 2`, no close dispatch, no detach" }
  no_op_deletion: { result: pass, deletion_justified_by_rca: true, detail: "the 12 removed lines of exit_tail are MOVED into the helper (same drain/close), not deleted; net +172/-20, adds behaviour (the reference release)" }
  adjacent_tests: { result: pass, suites_run: ["cargo xtask check rc=0", "cargo xtask clippy (x86-64) rc=0", "cargo xtask test rc=0 (port 232/0; arch leg skipped — no rustup)", "cargo xtask clippy --arch aarch64 rc=1 at fix AND base b82ee9d with identical diagnostics (4 pre-existing undocumented-unsafe errors)", "cargo test -p aarch64 --bins (darwin) 21/21; pre-existing sem_tests flake 1/500 at base and fix", "TCG boot-r9-smoke ALL STAGES PASS", "TCG boot-r9-net-smoke ALL STAGES PASS", "TCG boot-r9-tier1 ALL STAGES PASS 39 T1-OK + 23 T1-SKIP (incl. writeprobe)"], not_run: "cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu — the change is confined to aarch64/src (no port/, no x86_64/ source); x86-64 kernel ELF measured byte-identical base vs fix (sha256 817a422b…); a linux-gnu test binary cannot execute on this darwin host" }
  revert_and_reconfirm: { result: pass, bug_returned_on_revert: true, fixed_on_reapply: true, detail: "unfixed b82ee9d: rfdprobe -s backquote HANGS on TCG and HVF (procs 7+6 exited, no DIAGS); fixed b77f764 (= 3a1c9e5 bytes): `DIAGS rfdprobe: share ok` on TCG and HVF; default mode completes on both" }
  guardrail_verdict: accepted
  byte_identity: "aarch64 kernel-virt CHANGES: 57e2952 (pin) da4ab2da… → b82ee9d (20-08) e130d40a… → 3a1c9e5 2edf4f0e… (3a1c9e5 == tested b77f764, identical ELF). The raspi3b/4b configs share the aarch64 crate and change too (no Harvey gate). x86_64 kernel UNCHANGED (sha256 817a422b… at b82ee9d and the fix; no port/ or x86_64/ source changed) → no x64 FOD bump and no x64 gate series. HVF series needed: the three arm64 gates (boot-r9-smoke, boot-r9-net-smoke, boot-r9-tier1) at their recorded N-HVF at the pin that carries 3a1c9e5 — one set covers b82ee9d and 3a1c9e5 if the pin advances in one step."
files_changed: [r9 aarch64/src/proc.rs, r9 aarch64/src/syscall.rs]
related_findings_not_fixed:
  - "D2 (reading, not measured): bare rfork(RFCFDG) assigns the GLOBAL husk singleton empty_fdgroup() to a LIVE proc on both arches (x86_64 syscall.rs:1574, aarch64 b82ee9d) — later opens land in a table shared with every husk and every other RFCFDG caller and are never drained. Callers: usbd dev.c:160, libthread main.c:188, rc `rfork F`. Proposed: `Arc::new(Lock::new(\"fdgrp\", FdGroup::new()))` (changes x86_64 bytes)."
  - "aarch64 exit_tail's drain still closes under a BORROW (x86_64 fixed in ab0a977, 19-10 cont 6): duplicate devmnt fids inside one drained table never Tclunk; also the #44 husk-payload sweep + closepgrp mount half are unported to aarch64."
  - "pre-existing test flake proc::sem_tests::wake_consumes_picked_registrations_in_order (~1/500): swap_remove on the global SEM_WAITERS reorders another test's entries."
  - "same-class residual: other locals in frames that diverge into exit_tail (exec post-free kill, sys_exits' msg Vec) never drop — heap-only for the cases read."

## Prevention

human_verification: owner confirmed fixed (2026-11-19, relayed by the coordinator)

five_whys (branched):
  - code: why did the last sharer skip the drain? strong_count read 2 → why? the child's exit never released its ref → why? the detached Arc was a local of a `-> !` function, and Rust drops locals only on return/unwind → why was that possible? the sweep was written inline in the diverging tail with a comment ("the detached Arc just drops") that assumed scope-exit semantics the tail never reaches → actionable: keep every owning value out of diverging frames (sweep in a returning helper)
  - process/cross-arch: why did x86_64 have the fix and aarch64 not? c4f2f2c (#44) found the identical leak on x86_64 but framed it as a HEAP leak (churn gate), so it was ported as an x86_64 memory fix, not recognised as a correctness bug for any shared table, and no aarch64 follow-up was filed → actionable: a fix to a shared arch pattern names its sibling arch (ported or ledgered)
  - test/environment: why no failing gate before 20-07? no gate ran a process whose fd table is shared through plain rfork(RFPROC) under a reader waiting for EOF; rc, fork() and the tier-1 sweep all use RFFDG copies; libthread proccreate (the real exposure) is not in the arm64 sweep
why_not_caught: no gate existed for this class — the tier-1/smoke sweeps only exercise fork()'s RFFDG copies; the x86_64 #44 churn gate caught the same leak as heap growth but its fix was never mirrored to aarch64; and exit_tail can't be host-tested because it diverges
recurrence_guard: r9 3a1c9e5 — the sweep lives in a RETURNING helper (aarch64/src/proc.rs `release_fd_group`), and the host tests `proc::fdgrp_release_tests::{last_sharer_release_closes_the_shared_pipe_writer, unshared_group_release_closes_at_once, three_sharers_drain_only_at_the_third_release}` (verified passing; they kill the forget(detached) mutant). Harvey-side, rfdprobe -s stays in the image as the boot-level reproducer (`x=`{/bin/rfdprobe -s}` must complete). KB pattern: "locals in a diverging (`-> !`) kernel tail never drop — audit every owning value there".
