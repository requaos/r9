---
phase: 04-r9-userspace-exec-harvey-r9-abi
reviewed: 2026-11-02T00:00:00Z
depth: standard
files_reviewed: 41
files_reviewed_list:
  - build/aarch64/init.c
  - build/aarch64/mkpaqfs-host.sh
  - build/aarch64/README.md
  - build/aarch64/smokefork.c
  - build/aarch64/smokenote.c
  - build/aarch64/stage-build.sh
  - build/aarch64/tracehello.c
  - build/aarch64/trace-syscalls.py
  - .gitignore
  - aarch64/ABI.md
  - ../r9/aarch64/Cargo.toml
  - ../r9/aarch64/lib/config_virtexec.toml
  - ../r9/aarch64/lib/config_virtmulti.toml
  - ../r9/aarch64/lib/config_virtnotes.toml
  - ../r9/aarch64/lib/probe-initrd.txt
  - ../r9/aarch64/src/exec.rs
  - ../r9/aarch64/src/fault.rs
  - ../r9/aarch64/src/l.S
  - ../r9/aarch64/src/main.rs
  - ../r9/aarch64/src/pagealloc.rs
  - ../r9/aarch64/src/platform/raspi3b/devcons.rs
  - ../r9/aarch64/src/platform/virt/devcons.rs
  - ../r9/aarch64/src/platform/virt/timer.rs
  - ../r9/aarch64/src/pre_mmu/mod.rs
  - ../r9/aarch64/src/proc.rs
  - ../r9/aarch64/src/registers.rs
  - ../r9/aarch64/src/syscall.rs
  - ../r9/aarch64/src/trap.rs
  - ../r9/aarch64/src/trap.S
  - ../r9/aarch64/src/vm.rs
  - ../r9/HowItWorks.md
  - ../r9/port/src/dat.rs
  - ../r9/port/src/devcons.rs
  - ../r9/port/src/devproc.rs
  - ../r9/port/src/devroot.rs
  - ../r9/port/src/fdt.rs
  - ../r9/port/src/lib.rs
  - ../r9/port/src/paqfs.rs
  - ../r9/port/tests/paqfs_test.rs
  - ../r9/xtask/src/config.rs
  - ../r9/xtask/src/main.rs
findings:
  critical: 1
  warning: 8
  info: 8
  total: 17
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-11-02T00:00:00Z
**Depth:** standard
**Files Reviewed:** 41
**Status:** issues_found

## Summary

Reviewed the phase's full surface: the Harvey-side aarch64 harness (staged
init + smokes, staging/mkpaqfs host scripts, D-14 tracer), the frozen ABI
document, and the r9 kernel-side spine (paqfs reader, devroot/devproc-lite/
devcons devices, exec/loader, demand-fault handler, proc table + preemptive
scheduler, SVC dispatch with the measured syscall set, note delivery, trap
frame ABI and its asm, vm mapper, xtask harness).

The documented decisions recorded in `deferred-items.md` and the plan
deviations (the exec_bootstrap heisenbug containment, the
validate_user_range residency caveat, the 7c width-masking discipline, the
`Mode` bitflag replacement) were treated as settled and not re-litigated.
The code is unusually well-commented and the happy paths are carefully
constructed. The findings concentrate on the edges the comments claim are
handled: a silent data-corruption hole in the demand pager's segment
classification, an invariant violation in the vm mapper's error path,
lifecycle gaps on failure paths (rfork, RFNOWAIT, fault-kill), and two
user-reachable kernel-panic/DoS shapes (one via the noted() restore, one
via the fault handler's kernel-address panic).

The headline issue is CR-01: the data/bss page-sharing hole is a latent
silent-corruption defect in the shipped demand pager that the current
smokes happen not to trigger; the first Phase-5 binary that touches bss
before the tail of its last data page will read zeros where file bytes
belong.

## Critical Issues

### CR-01: Demand pager silently corrupts data when bss faults precede data on the shared last data page

**File:** `../r9/aarch64/src/fault.rs:93-103` (classification/guard) and `../r9/aarch64/src/exec.rs:142-151` (segment ranges)

**Issue:** Segment ranges are recorded with byte-exact ends:
`data = [data_va, data_va+data)`, `bss = [data_end, bss_end)` (exec.rs:143-148).
`data_va` is 64K-aligned but `data_end` is arbitrary, so whenever
`data % 4096 != 0 && bss > 0` the last data page and the head of bss share
one 4K page. The fault handler classifies by faulting *address* and pages
whole pages:

- If a **bss** address in the shared page faults first, `classify` returns
  `Bss` and `page_in_zeroed` maps the page zeroed (fault.rs:196-200). The
  data file bytes at `[page_va, data_end)` are never loaded — subsequent
  reads of those data variables return zeros. The page is now mapped, so no
  later fault can repair it. **Silent userland data corruption.**
- (The reverse order — data first — happens to be safe because
  `page_in_from_image` zero-fills the page and clamps the read at
  `content_end`, which was the 04-06 Rule-1 fix. Only the bss-first order
  corrupts.)

The shipped binaries don't trip it (their access order touches the data
tail first), which is why the gates are green; it is a property of the
binary layout and access order, not of the harness. Any Phase-5 binary with
a bss object in the shared page touched before the data tail (e.g. a large
bss array whose first element shares the page) corrupts silently.

**Fix:** Classify by *page*, not by address: record the data segment's
classification end as `(data_end).next_multiple_of(PAGE_SIZE_4K)` so the
shared page classifies as `Data` and `page_in_from_image`'s existing
zero-fill + `content_end` clamp yields exactly the right bytes (file bytes
below `data_end`, zeros above). Keep the bss *range* byte-exact for
`validate_user_range`. E.g. in `install_image`:

```rust
let data_classify_end = data_end.next_multiple_of(PAGE_SIZE_4K).min(bss_end);
```

and use a separate page-granular classification map (or round the data
range and rely on the clamp). Alternatively, teach `page_in_zeroed` to
splice the file window from `proc.text_source` when the Bss/Stack page it
maps overlaps `[data_va, data_end)` — but the classification fix is smaller
and matches the existing clamp machinery.

## Warnings

### WR-01: `map_to` error path abandons the recursive slot-511 restore

**File:** `../r9/aarch64/src/vm.rs:428-454`

**Issue:** `map_to` swaps the target table into the active root table's
recursive slot 511 (`replace_recursive_entry`), then walks. Its own comment
says "We *must* return it to its original state on exit" — but on walk
failure the code prints and `return Err(err)` at line 453 **without** the
`write_recursive_entry(pgtype, old_recursive_entry)` restore that the
success path performs at line 462. The active table's slot 511 is left
pointing at the *other* (e.g. inactive proc) root table until the next
`map_to` re-points it. Each later `map_to` reads the corrupted slot as its
"old" entry and restores the corruption, so the recursive self-map is
permanently wrong outside the dance — any future direct recursive-window
access resolves through the wrong root. Triggered by `EntryIsNotTable` or a
pagealloc failure mid-walk.

**Fix:** Restore the recursive entry on every exit. Restructure as

```rust
let result = (|| { /* walk + write dest entry */ })();
vmtrait_impl.write_recursive_entry(pgtype, old_recursive_entry);
result
```

(or a guard value whose Drop restores slot 511).

### WR-02: rfork_clone / new_user_proc error paths leave a runnable null-frame child and leak pages

**File:** `../r9/aarch64/src/proc.rs:839-891` (rfork_clone), `:498-511` (table-full path)

**Issue:** Two failure-path defects in process construction:

1. `new_user_proc` inserts the child into `PROC_TABLE` with
   `state: Ready` and `frame: ptr::null_mut()` (proc.rs:505-511). If
   `rfork_clone` then fails mid-copy (`rfork: page alloc failed`,
   `rfork: child user map failed`, `rfork: kernel alias map failed`), it
   returns `Err` with the half-built child still Ready in the table. The
   scheduler's `pick_next` will eventually pick it and produce
   `Resume::Return(null)` → `trap_return` reads `CURFRAME = 0` →
   `mov sp, x9` with x9=0 → load from VA 264 → EL1h abort → kernel panic.
   Loud rather than silent, but a scheduler-reachable landmine left by an
   ordinary OOM.
2. On `proc: table full` (proc.rs:502-503), the already-allocated root
   table page and kstack pages (and their kernel mappings) are leaked.
   Reachable with `MAX_PROCS = 8` once real init spawns grandchildren.

**Fix:** On any `rfork_clone` error after `new_user_proc`, mark the child
`Moribund` (or remove the slot) and free its recorded `kstack_pas` + root
page via the same PAs `free_root_table_and_kstack` uses; free the copied
user pages already in `child.mapping_log`. On table-full, free the root
page + kstack pages before returning the error.

### WR-03: RFNOWAIT child exit corrupts the parent's AWAIT pivot forever

**File:** `../r9/aarch64/src/proc.rs:924` (increment), `:810-829` (post_wait_record), `../r9/aarch64/src/syscall.rs:611-631` (sys_await)

**Issue:** `rfork_clone` unconditionally does `parent.children_live += 1`.
`children_live` is decremented only in `sys_await` when a wait record is
consumed (syscall.rs:613). An RFNOWAIT child's EXITS posts *no* record
(`if !proc.nowait`, syscall.rs:671-673), so the parent's `children_live`
stays elevated for the rest of the parent's life. A subsequent AWAIT with
no queued records takes the `children_live > 0` branch and blocks in
`sched()` — and nothing will ever wake it, because the nowait exit also
skips the wake. Result: a legal RFNOWAIT sequence permanently hangs the
parent's next AWAIT (even after every real child has exited). This
contradicts the documented claim (syscall.rs:535-536, HowItWorks.md 04-07
section) that "RFNOWAIT is honored".

**Fix:** Either don't increment `children_live` for nowait children
(`if !nowait { parent.children_live += 1 }` — matching "not reaped by
AWAIT" semantics), or decrement it in `exit_tail` for nowait children. The
former matches Plan 9, where an RFNOWAIT child is invisible to pwait.

### WR-04: NOTED's NCONT/NRSTR restores `psr` from user-writable memory without sanitizing to EL0t

**File:** `../r9/aarch64/src/syscall.rs:305-348`

**Issue:** The saved Ureg (`nureg`) lives on the user stack, pushed there by
`deliver_notes`, and the handler can legally edit it (that is the documented
`notejmp` contract). `sys_noted` validates the saved `pc` (aligned, in text)
and `sp` (aligned, valid range) but then `copy_nonoverlapping`s the full
280 bytes — including `psr` at offset 264 — over the live frame. A handler
that sets the saved psr's M field to EL1h (`0x5`) makes `trap_return`'s
`msr spsr_el1` + `eret` (trap.S:173-181) elevate to EL1 at the
(text-validated, so PXN-protected) user pc → EL1 instruction-abort → the
EL1 panic path → kernel halt. User-triggerable kernel panic = DoS. (Full
privilege escalation is contained by PXN on user text; the psr/`type`
fields are still kernel state crossing the trust boundary unchecked.)

**Fix:** After the copy, force the privilege bits:
`frame.psr &= !0xf` (EL0t) — and consider pinning `type` to a kernel value
or documenting it as informational-only (ABI.md already does the latter).
The donor's noted() validates pc/sp; r9 already exceeds that — add the psr
mask to make the restore boundary closed.

### WR-05: deliver_notes has no in-handler guard — a second note overwrites `note_ureg` mid-handler

**File:** `../r9/aarch64/src/trap.rs:332-389`

**Issue:** The donor's `donotify` refuses to deliver while `up->notified`
is set (a note handler is active); r9 has no equivalent check. If two notes
are queued (NOTE_QUEUE_CAP = 8 permits it), the second is delivered at the
next return-to-EL0 boundary even while the first handler is still running:
`proc.note_ureg` is overwritten with the second note's saved Ureg and
`proc.last_note` is replaced. When the *first* handler finally calls
`noted(NCONT)`, `sys_noted` restores from the *second* note's frame —
resuming the wrong context with the wrong pc/sp. The smoke only ever posts
one note, so the gate can't see this.

**Fix:** At the top of `deliver_notes`, if `proc.note_ureg != 0` (or a
dedicated `notified` flag), leave the note queued and return — delivery is
re-attempted at the next boundary (noted() clears `note_ureg`, unblocking
it). This matches donor semantics and requires no new queue machinery.

### WR-06: fault-kill paths bypass the 04-07 lifecycle — parent AWAIT hangs, kernel wedges with IRQs masked

**File:** `../r9/aarch64/src/fault.rs:275-284` (kill_current_process), `../r9/aarch64/src/trap.rs:441-450` (other-EL0-sync kill)

**Issue:** `kill_current_process` is the single-process-era "kill": mark
Moribund, print, park in `wfi` — with IRQs masked (trap context), so the
`wfi` never wakes; the whole system hard-wedges. Post-04-07 this is stale:
(a) the comment "the faulting process is the only one" is no longer true
(MAX_PROCS = 8, init + 3 children); (b) no wait record is posted to the
parent, so a faulting child leaves the parent's AWAIT blocked with
`children_live > 0` — but the wedge fires first, so the hang is moot; the
wedge itself is the defect: one process's wild pointer takes down every
sibling and the scheduler. Neither `deferred-items.md` nor the 04-07/04-08
docs record this as an accepted limit (the kill's own comment still claims
the single-process era).

**Fix:** Route the kill through `syscall::exit_tail(proc, b"fault")` (or a
fault-named message), which posts the wait record, wakes the parent, frees
by the recorded PAs, and hands off to `sched_from_moribund` — exactly the
shape deliver_notes's no-handler kill already uses (trap.rs:344-351). The
all-exited halt remains as exit_tail's tail.

### WR-07: exec_replace's "seed can no longer fail" invariant is false — post-free failure strands the proc

**File:** `../r9/aarch64/src/exec.rs:215-237`

**Issue:** The T-04-13 ordering comment says "Every remaining fallible check
runs BEFORE the free … (seed_user_pages can no longer fail once this
passes)". But `seed_user_pages` → `map_one_user_page` still calls
`pagealloc::allocate_physpage()` and `map_to` — both fallible (OOM /
EntryIsNotTable) — *after* `proc.free_user_mappings()` has destroyed the
old image. On that Err, `sys_exec` returns -1/errstr to a trap frame whose
pc/sp name freed text pages; the next fetch faults into the *new*
half-installed segment map and the process limps or dies messily. The
documented mitigation ("parse first, free second, seed third — validation
failure returns -1 with the old image intact") holds only for the pre-free
checks, not for the seed's allocation failures.

**Fix:** Either allocate the two seed pages (and preflight the two `map_to`
table allocations) before freeing, or accept the reality and make the
post-free failure path diverge loudly (kill via exit_tail) instead of
returning -1 to freed text. At minimum, correct the comment so the next
reader doesn't rely on the false invariant.

### WR-08: user fault on a top-half VA panics the kernel (user-reachable DoS)

**File:** `../r9/aarch64/src/fault.rs:69-76`

**Issue:** `if va >= KZERO { panic!("user fault: kernel-context address …") }`
checks the *user's* FAR. An EL0 process that dereferences a wild high
pointer (e.g. `0xffff8000_00000000`) takes a translation fault with exactly
that FAR — no kernel bug involved — and the kernel panics and halts. The
intent (T-04-09) is to catch the *handler* touching kernel-context memory,
but the guard runs on the faulting user address, turning an ordinary
userland wild-pointer bug into a kernel panic. Not reachable from the
measured binaries; reachable from any future binary with a bad pointer.

**Fix:** Only panic when the trap came from EL1 (the frame's `psr & 0xF != 0`
— but this handler is only routed lower-EL aborts, so that never happens);
for EL0-origin faults, a high FAR is just an unresolvable user fault: fall
through to `unresolvable(frame, class, far, access)` and kill the process.

## Info

### IN-01: Stale tick-rate comment in the scheduler header

**File:** `../r9/aarch64/src/proc.rs:531-532`
**Issue:** Comment says "100 Hz = 10 ms this era"; timer.rs:29 sets
`TICKS_PER_SEC = 1000` (1 ms, the 04-07 D-13 change). HowItWorks.md has the
correct 1000 Hz.
**Fix:** Update the comment to 1000 Hz / 1 ms.

### IN-02: Stale "never freed in the single-process era" comment on Proc's raw-pointer fields

**File:** `../r9/aarch64/src/proc.rs:148-150`
**Issue:** 04-06/04-07 free the root-table page and kstack pages in
`free_root_table_and_kstack` (EXITS/EXEC paths); the struct comment still
says "never freed".
**Fix:** Reword to name the actual lifetime (freed by the exit/exec paths
via the recorded PAs).

### IN-03: Brace + first statement jammed on one line

**File:** `../r9/aarch64/src/fault.rs:196`
**Issue:** `fn page_in_zeroed(...) {    let pa = …` — formatting slip
inconsistent with the file.
**Fix:** Newline after `{`.

### IN-04: devcons pread advances the streaming offset even for explicit-offset reads

**File:** `../r9/aarch64/src/syscall.rs:245-251`
**Issue:** The DEV_CONS arm does `chan.offset += read` unconditionally.
devroot's read advances the offset only when `offset == u64::MAX` (the -1
streaming convention, devroot.rs:132-136); devcons should match — offsets
are meaningless on the console, so impact is nil, but the two devices now
disagree on the convention.
**Fix:** Gate the advance on `off == u64::MAX` like devroot.

### IN-05: map_one_user_page comment says "(inactive)" but exec_replace calls it on the active table

**File:** `../r9/aarch64/src/exec.rs:318-321`
**Issue:** The doc comment describes the not-yet-active first-exec case;
04-06's exec_replace reuses it on the caller's live TTBR0 table (map_to's
dance handles both, as free_user_mappings' comment notes).
**Fix:** Note both call shapes in the comment.

### IN-06: trace ENTRY_RE silently drops zero-argument syscall fragments

**File:** `build/aarch64/trace-syscalls.py:94`
**Issue:** `ENTRY_RE = ^(\d+) (\S+) (\S+) [0-9a-f]+\s` requires whitespace
after the bare-hex pc. A syscallfmt entry fragment for a zero-arg syscall
(e.g. `nsec`) ends at the pc with no trailing space, so it never matches
and would be silently absent from the measured set that drives r9's
dispatch table. Today's set ({pwrite, open, pread, close, exits}) all carry
args, so the artifact is unaffected — but the parser would under-report if
the traced binary's footprint ever grows a zero-arg call.
**Fix:** Make the tail `[0-9a-f]+(?:\s|$)`.

### IN-07: trace Log ring trim invalidates absolute search bases

**File:** `build/aarch64/trace-syscalls.py:349-380`
**Issue:** `Log.write` trims the buffer to the last 512 KiB once it exceeds
1 MiB, but `search_from`/`match_from` take absolute offsets captured via
`log.length()` earlier (`base = log.length()` at :549, :567, :584). After a
trim, those bases point 512 KiB too far into the kept window and markers
are missed (the run then times out rather than asserting falsely). Captures
this era are ≪ 1 MiB, so latent.
**Fix:** Track a total-bytes counter for bases, or store `(absolute_base,
buf_start_offset)` and rebase on search.

### IN-08: sha256 fallback only covers spawn failure, not tool failure

**File:** `../r9/xtask/src/main.rs:1696-1707`
**Issue:** `or_else` falls back to `shasum` only if spawning `sha256sum`
errors; if `sha256sum` exists but exits nonzero, the function returns Err
without trying `shasum`.
**Fix:** Retry with `shasum -a 256` when `!out.status.success()` too.

---

_Reviewed: 2026-11-02T00:00:00Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
