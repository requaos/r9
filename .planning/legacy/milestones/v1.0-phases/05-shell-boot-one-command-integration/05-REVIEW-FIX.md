---
phase: 05-shell-boot-one-command-integration
fixed_at: 2026-11-03T23:00:53Z
review_path: .planning/phases/05-shell-boot-one-command-integration/05-REVIEW.md
iteration: 1
findings_in_scope: 10
fixed: 10
skipped: 0
status: all_fixed
---

# Phase 5: Code Review Fix Report

**Fixed at:** 2026-11-03T23:00:53Z
**Source review:** .planning/phases/05-shell-boot-one-command-integration/05-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 10 (fix_scope `critical_warning`: 2 Critical + 8 Warning; the 13 Info findings are out of scope)
- Fixed: 10
- Skipped: 0

Nine fixes are in the sibling r9 repo (`/Users/neilskinner/code/r9`, branch
`main`), one in the Harvey repo (branch `master`) — committed directly in each
owning repo per the two-repo phase convention (orchestrator project context:
no worktrees, no branches, `git -C` from the Harvey root). Commits are not
pushed.

**Where verification ran:** all gates ran in the main checkouts (no worktree —
the two-repo phase commits fixes directly in the owning repo; the r9 gates
need the nix dev shell, qemu, and the warm `target/`, which a hand-rolled
worktree does not have). Host gates per fix: `nix develop ... cargo test -p
port` (61 → 76 tests across the series as new pins landed) and, for arch-side
changes, `cargo xtask build --arch aarch64 --config virtshell` (the gate's
config — a bare `cargo check` builds the default raspi3b platform and does
not type-check virt/dev_exec code). Boot gate: `nix run --override-input
r9 git+file:///Users/neilskinner/code/r9 .#boot-r9-smoke` (the D-13 dev-flow
override so the locally committed kernel is the one tested — the flake's r9
input otherwise pins an upstream rev). See "Boot-gate status" below.

## Fixed Issues

### CR-01: Six devices serve malformed stat(5) records (no muid string)

**Files modified:** `../r9/port/src/devroot.rs`, `../r9/port/src/devcons.rs`,
`../r9/port/src/devenv.rs`, `../r9/port/src/devpipe.rs`,
`../r9/port/src/devdup.rs`, `../r9/port/src/devsrv.rs`, `../r9/port/src/dat.rs`;
tests `../r9/port/tests/devenv_test.rs`, `devroot_rawimg_test.rs`,
`devcons_test.rs`, `devpipe_test.rs`, `devdup_devsrv_test.rs`
**Commit:** a244ba2 (r9)
**Applied fix:** appended the fourth length-prefixed string (muid) in every
device's stat record writer and corrected the `need` arithmetic to
`41 + (2 + name.len()) + 3 * (2 + 6)` (plus the `listing.resize(...)`
pre-sizing in the five directory-listing arms). muid = uid per the donor
devdir (`db->muid = user` — verified in `sys/src/9k/port/dev.c:41`): "glenda"
for the synthetic devices (D-02), the PaqDir's uid for devroot. dat.rs's
StatWriter doc comment corrected (three → four strings).
**Test re-pinning (per the review's note that the suite encoded the defect):**
the two tests that pinned the OLD 3-string byte count with a "convD2M"
mislabel (`devenv_test.rs:294`, `devroot_rawimg_test.rs:239`) now walk all
four strings, require the record to end exactly after them (statcheck's
`buf != ebuf` rejection), and pin the 4-string count. Every stat-serving
device's suite now has a statcheck-shaped 4-string round-trip pin like
devproc's parse_dir: new `stat_records_carry_four_strings` tests in
devcons_test.rs and devpipe_test.rs, listing+stat walks added to
devdup_devsrv_test.rs, the extended assertions in devenv_test.rs and
devroot_rawimg_test.rs.
**Verification:** `cargo test -p port` 69/69 green (was 61; +8 from the new
pins), including the previously-mispinned suites.

### CR-02: sys_stat/sys_fstat skip the cold-buffer residency step

**Files modified:** `../r9/aarch64/src/syscall.rs`
**Commit:** 09eea41 (r9)
**Applied fix:** added `fault::ensure_user_resident(proc, buf_va as usize,
(buf_va + n) as usize)` immediately after `validate_user_range` in both
handlers — the exact line the seven sibling buffer-writing handlers carry
(sys_pread's shape), so an in-segment but never-demand-faulted heap page
(libc dirstat's malloc'd Dir buffer straddling into an untouched brk_ page)
can no longer take an EL1h data abort mid-write (the trap.rs dump-and-halt
arm — a kernel halt on a legal user syscall).
**Verification:** `cargo check -p aarch64 --target aarch64-unknown-none`
clean. Boot-gate status: see below.

### WR-01: consctl parser silently ignores newline-terminated words

**Files modified:** `../r9/port/src/devcons.rs`, `../r9/port/tests/devcons_test.rs`
**Commit:** 363a0ec (r9)
**Applied fix (donor-fidelity judgment):** the review claimed the donor's
conswrite tokenizes via parsecmd; the actual 9k donor
(`sys/src/9k/port/devcons.c`, the Qconsctl write arm) scans space-separated
words with PREFIX compares (`strncmp(a, "rawon", 5)`) — which is exactly what
makes the natural `echo rawon >#c/consctl` work there (the word is
`"rawon\n"`, and strncmp matches the first 5 bytes). The r9 whole-word
compare diverged from the donor, so it is fixed to donor semantics:
`w.starts_with(b"rawon")` / `w.starts_with(b"rawoff")` per space-separated
word, preserving the donor's prefix quirk.
**Verification:** `cargo test -p port --test devcons_test` green with new
pins: newline-terminated `rawon\n`/`rawoff\n` switch the mode (the mode word
reads back `raw\n`/`cooked\n`).

### WR-02: sys_sleep wraps negative arguments into ~49-day sleeps

**Files modified:** `../r9/aarch64/src/syscall.rs`
**Commit:** 9c990c5 (r9)
**Applied fix (donor-fidelity):** the donor's syssleep
(`sys/src/9k/port/sysproc.c:633`) is signed — `if(ms <= 0) yield();` — so the
r9 unsigned read diverged. The handler now reads the slot as `i32`; the
`ms <= 0` arm preempts and returns 0 (subsuming the existing sleep(0) yield,
same behavior — the donor yields for all n <= 0).
**Verification:** `cargo check -p aarch64 --target aarch64-unknown-none`
clean. Boot gate: see below.

### WR-03: cooked-mode ^D on an empty line never yields EOF

**Files modified:** `../r9/port/src/devcons.rs`, `../r9/port/tests/devcons_test.rs`
**Commit:** 065a358 (r9)
**Applied fix (donor-fidelity):** the donor's consread sends the accumulated
line WITHOUT the ^D byte even when empty — `qwrite(lineq, kbd.line, 0)` —
whose qread returns 0 (the EOF that lets rc exit on ^D at a fresh prompt;
verified in `sys/src/9k/port/devcons.c:869-881`). A flat byte queue cannot
represent a zero-length block, so the CTLD arm now records a `pending_eof`
count for empty-line ^Ds, consumed by the reader only once `lines` has
drained (preserving the donor's block order: bytes queued before the ^D read
first). The wake path is unchanged (the ^D byte is itself an RX byte, so the
tick's cons_rx_pending check readies the blocked reader). Module header's ^D
contract note updated.
**Verification:** `cargo test -p port --test devcons_test` green with new
pins: empty ^D reads 0 (EOF), the next line still reads, and
queued-bytes-then-EOF ordering holds.

### WR-04: pgrp lock held across blocking devmnt RPCs in most namespace syscalls

**Files modified:** `../r9/port/src/namec.rs`, `../r9/port/src/pgrp.rs`,
`../r9/aarch64/src/syscall.rs`; tests `../r9/port/tests/namespace_test.rs`,
`devmnt_test.rs`, `devenv_test.rs`, `devdup_devsrv_test.rs`
**Commit:** 36dcc39 (r9)
**Applied fix:** restructured per the review's sys_mount discipline, applied
uniformly. `namec` now takes `&Lock<Pgrp>` and consults the mount table via
per-consult chain SNAPSHOTS — new `pgrp::chain_snapshot` dup()s the chain's
chans under a scoped lock (deep_copy's shape; the pipe open-end reference
rule rides dup), then drops the lock BEFORE any device walk. A devmnt Twalk
RPC that blocks mid-syscall now never runs with the pgrp lock held, so a 9P
server proc sharing the pgrp (the fork()ed paqfs, no RFNAMEG) can take the
lock while serving without deadlocking the system. The seven handlers keep no
guard across resolution/open; bind/unmount/mount/create take the lock only
for the mount-table mutation (sys_create's dot-redirect consult snapshots,
drops, then fid-clones). Tests wrap their Pgrp in mcslock::Lock to match.
**Verification:** `cargo test -p port` 74/74 green (all namespace/devmnt
union-matrix and fid-aliasing pins intact); `cargo xtask build --arch
aarch64 --config virtshell` clean. **Boot gate:** the first full smoke
against the fixed tree exposed a LATENT allocator defect
(`QuickFit::unlink` dropped the list head on mid-list removals and wiped
the list on no-match walks — a free-list cycle under churn) that WR-04's
added snapshot allocations steered the gate into; root-caused via an
instrumented bisect and fixed as `ec8833c`. A residual nondeterministic
lost-byte wedge in the 05-08 RX path (untouched by this series, exposed by
its timing shifts) was then closed by the tick's direct FIFO poll
(`d8496d3`). See "Boot-gate status" for the full hunt and tally.

### WR-05: new_user_proc leaks the root-table and kstack pages on mid-construction failure

**Files modified:** `../r9/aarch64/src/proc.rs`
**Commit:** 26db312 (r9)
**Applied fix:** all three mid-construction failure arms (kstack VA
contiguity, kstack page alloc, kstack map) now funnel through a shared
`unwind_new_proc` — the table-full arm's frees refactored into it (same
messages via a context tag). The map-failure arm records the page in
kstack_pas BEFORE the map so the unwind covers it. The kernel-side VA
mappings stay, per the free_root_table_and_kstack convention. The kstack
loop is an index loop now (the unwind slices kstack_pas[..n], which
iter_mut's borrow forbids).
**Verification:** `cargo check -p aarch64 --target aarch64-unknown-none`
clean. Boot gate: see below.

### WR-06: exec accepts truncated binaries (zero-filled text tail) and devmnt read_all rejects an exact-cap file

**Files modified:** `../r9/aarch64/src/exec.rs`, `../r9/aarch64/src/proc.rs`,
`../r9/port/src/devmnt.rs`, `../r9/port/tests/devmnt_test.rs`
**Commit:** 667e4cc (r9)
**Applied fix:** (a) `read_aout` now checks `HDR_SIZE + text + data + syms`
against the backing's length (new `ByteBacking::file_len` — the paqfs
entry's recorded length / the staged buffer's length) and errs "exec:
truncated binary" up front, before any mapping exists (the donor fails the
exec on a short segment read; r9's pager clamps at EOF and zero-fills, which
is why the up-front check is the right port). syms ride the image's length
semantics though exec never pages syms in. (b) `read_all`'s cap check moved
after the read: reads may reach cap+1, so an exactly-cap file terminates via
the empty-read EOF arm and only a byte past the cap errors "file too large".
**Verification:** `cargo test -p port --test devmnt_test` 10/10 green (new
pins: exact-cap stages whole, one-byte-over errors with the named errstr,
wide-cap stages the same bytes); `cargo check -p aarch64` clean. Boot gate:
see below.

### WR-07: pkill teardown patterns kill foreign r9 qemu processes

**Files modified:** `nix/smoke-r9.sh`, `build/aarch64/drive-r9.expect`,
`build/aarch64/smoke-r9.expect`, `build/aarch64/run-r9.sh` (Harvey repo)
**Commit:** 0f70588e7 (harvey)
**Applied fix:** a per-run unique tag now rides qemu's argv as `-name`:
`R9_RUN_TAG="r9smoke-$$-$(date +%s)"` exported by smoke-r9.sh (the driver
adds it to the direct qemu spawn; standalone driver runs default to
`r9smoke-[pid]-[clock seconds]`), and drive-r9.expect sets
`r9drive-[pid]-[clock seconds]` into the spawned run-r9.sh's environment
(run-r9.sh honors R9_RUN_TAG → `-name`). Both survivor sweeps pkill on
`[q]emu-system-aarch64.*<tag>` — bracket-fixed against self-match, scoped to
THIS run; the kernel-name-class patterns are gone, so a dev's interactive
boot-r9 or a parallel CI job is never matched. The expect drivers'
process-group kills are unchanged (already per-run). Tcl gotcha found and
fixed during implementation: the pattern's bracket must be written `\[q]`
inside Tcl double quotes (a bare `[q]` command-substitutes at exec time —
measured "invalid command name q").
**Verification:** `sh -n` clean on both scripts; brace-balance clean on both
expect drivers; live pkill probe (qemu-argv stand-ins) kills the tagged
process while a foreign same-kernel-class process survives. End-to-end: the
WR-07 tag is visible in the gate's qemu argv
(`-name r9smoke-30252-1793732223` in the verification runs' `ps` output) and
the bisect/gate runs below did not kill each other's qemu despite overlap
discipline being available.

### WR-08: devroot directories are unreadable

**Files modified:** `../r9/port/src/devroot.rs`, `../r9/port/tests/devroot_rawimg_test.rs`
**Commit:** 9592d17 (r9)
**Applied fix (donor-fidelity):** the donor's devroot serves directory
listings (devdirread over the root tree), so the error arm diverged.
`DevRoot::read` now serves the stat(5)-wire listing of the PaqDir's children
via the paqfs reader's `read_dir`, one 4-string record per child through a
shared `write_entry` (stat delegates to it — the record shape is
single-sourced; CR-01's muid rule included). The root listing appends the
synthesized /rootfs.img node (walk synthesizes it there; a listing names
every walkable entry), sharing the node's PaqDir construction
(`rawimg_paqdir`) with walk's chan synthesis. Module header updated.
**Verification:** `cargo test -p port` 75/75 green (new
`dir_reads_serve_the_four_string_listing`: root listing names
[boot, rootfs.img] in records that end exactly after four strings, explicit
offset windows slice the listing, subdirectory listings carry no raw node).

## Skipped Issues

None — all in-scope findings were fixed.

## Out-of-scope note (Info tier, not fixed per fix_scope)

IN-05 (`trace-syscalls.py` leaked a socket per poll iteration) references the
Python harness that was deleted today in favor of `build/harness/` (Rust).
The replacement was checked for the same defect class per the orchestrator's
instruction: `build/harness/src/trace.rs:966,1613` probe readiness with
`std::net::TcpStream::connect(...)` whose stream drops (closes) at the end of
each probe expression, and `:1069,1712` use `UnixStream::connect` the same
way — RAII closes every probe socket deterministically. The defect does not
exist in the replacement. No action taken (Info tier + target file deleted).

## Boot-gate status

**Regression found by the gate and fixed during this run.** The first two
`nix run --override-input r9 git+file:///Users/neilskinner/code/r9
.#boot-r9-smoke` runs against the fully-fixed tree (9592d17) failed
identically at the final stage: every stage through DEL safety and the typed
`exit` passed, then the restarted rc (pid 11) was scheduled
("sched: switch 1->11") but produced no further output — no page-in faults,
no prompt — to the 600s restart-prompt timeout. The identical shape across
two runs ruled out the flake class (the 05-01 note-timeout flake was
root-caused and buried 40/40; no restart-stage flake is on record).

**Bisect** (smoke at pinned revs): 065a358 (through WR-03) PASSED all stages;
36dcc39 (WR-04 on top) FAILED identically — isolating the trigger to WR-04.

**Localization** (temporary instrumented commits 0336a0e/4ae0462 — sys_exec
stage prints + one line per 9P transact + an allocator free-list spin guard;
the instrumentation was reverted in the fix commit): pid 11's exec resolved
and opened /bin/rc through the mount union (namec/Twalk/Topen all answered),
then stalled in `devmnt::read_all`'s staging loop after the 17th reply — the
code between replies is only heap traffic, and the spin guard caught
`QuickFit::unlink` walking a CYCLED misc free list (node size 260608, the
freed staging-buffer class).

**Root cause (fixed as commit ec8833c):** `QuickFit::unlink` advanced the
`list` cursor during the walk and returned it as the list on a match — so a
mid-list removal returned the *just-removed node* as the new head (orphaning
the real head and tail, and leaving the still-hashed header on the free
list; the same header in two lists cycled on a later insert), and a no-match
walk returned `None` (wiping the entire free list — a permanent leak).
WR-04's per-consult namec snapshots added the heap churn that steered the
gate into the latent corruption; the allocator bug itself predates the
review and is caller-independent. The fix preserves the head in all three
arms, plus clamps `align_prefix`'s alignment offset to the prefix length (a
zero-length prefix behind a non-64-aligned tail cursor tripped the
`offset <= len` assert — a legal pattern the gate trajectory never produced;
found by the new regression test's non-64-multiple sizes). A host-side
regression test (`misc_midlist_removal_preserves_the_list`, pointer-identity
pins on both arms) was verified RED against the old unlink shape before
going green; `cargo test -p port` 76/76 green.

**Root cause 2 (fixed as d8496d3):** with the allocator fixed, a residual
NONDETERMINISTIC, stage-moving wedge remained (restart-print, cat-prompt, and
ls-prompt across runs — ~40% at ec8833c/32e86be). The one fully-traced
failure (instrumented run at 7b0d948) showed rc's cons read wake on a typed
command's byte PREFIX, find no completable line, and re-block forever — a
byte that misses its RX/RT interrupt is lost until the next byte arrives.
The RX path is 05-08 code the review series never touched; the class is
host-pacing-sensitive, which is why the series' timing shifts moved its
probability (and why the baseline's 3/3 pass did not clear it). The 1 kHz
tick now drains FIFO-held bytes into the cons ring before the wake check
(`pl011::cons_rx_poll` — the IRQ handler's own drain discipline, safe from
tick context: IRQ-masked, same-priority IRQs never nest), so a lost
interrupt costs one tick of latency instead of wedging the reader forever.

**Permanent diagnostics retained (zero cost unless triggered):** the
allocator free-list spin guard (a cycled list panics loudly — the gate's
panic marker catches it — instead of hanging silently), and a scheduler
idle-arm wedge tripwire (24c3b92, folded into d8496d3): 2000 consecutive
idle iterations with a cons-waiting Blocked reader while the ring/FIFO holds
bytes — or ~100 s of full idle — dumps the proc table, the cons input state,
and the ring/FIFO positions to the wire (the autopsy evidence for any future
gate wedge).

**Gate tally (all runs the full default pipeline, `nix run --override-input
r9 … .#boot-r9-smoke`):**
- baseline 065a358 (pre-WR-04): 3/3 PASS
- 36dcc39 / 9592d17 (WR-04 landed, allocator bug live): 0/3 — deterministic
  restart-prompt wedge (the allocator cycle, proven by the spin guard)
- ec8833c / 32e86be (allocator fixed): 1/4 — moving wedges, no cycle
- 24c3b92 (+ wedge tripwire): 4/4 PASS (tripwire never fired)
- d8496d3 (+ tick FIFO poll; final state): **3/3 PASS** — the phase's
  two-consecutive gate standard exceeded; the retained run-3 capture is
  `05-REVIEW-FIX-smoke.log` in this directory (113,162 bytes — banner →
  paqfs → stacksmoke → prompt → echo → ls → ps → cat → DEL → exit →
  restart all present; zero wedgedbg/allocator tripwire lines).

The four debug commits (0336a0e, 4ae0462, 29651e3, 7b0d948) remain in r9
history as the investigation record; their instrumentation is fully reverted
in the final state (diff 9592d17..d8496d3 adds only the two tripwires and
the tick FIFO poll beyond the fixes themselves).

**Note for the verifier:** a bare `cargo check -p aarch64 --target
aarch64-unknown-none` builds the DEFAULT raspi3b platform (r9's
platform/mod.rs) and does NOT type-check virt/dev_exec code; the gate's
config is checked with `cargo xtask build --arch aarch64 --config
virtshell`. Every arch-side change in this series was verified with the
latter.

_The per-finding host gates (cargo test -p port 76/76, cargo xtask build of
the virtshell config) are green for every commit listed above._

---

_Fixed: 2026-11-03T23:00:53Z_
_Fixer: the agent (gsd-code-fixer)_
_Iteration: 1_
