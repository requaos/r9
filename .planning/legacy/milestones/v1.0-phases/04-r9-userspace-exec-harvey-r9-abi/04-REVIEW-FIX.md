---
phase: 04-r9-userspace-exec-harvey-r9-abi
fixed_at: 2026-11-02T20:06:51Z
review_path: .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 9
skipped: 0
status: all_fixed
---

# Phase 4: Code Review Fix Report

**Fixed at:** 2026-11-02T20:06:51Z
**Source review:** .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 9 (fix_scope `critical_warning`: 1 Critical + 8 Warning; the 8 Info findings are out of scope)
- Fixed: 9
- Skipped: 0

All fixes are in the sibling r9 repo (`/Users/neilskinner/code/r9`, branch `main`),
committed there per the two-repo phase convention. No Harvey-side files were
modified (the only Harvey-side findings, IN-06/IN-07, are Info-tier and out of
scope). The frozen trap-frame ABI was not touched: no `TrapFrame` field or
offset changed; `cargo xtask ureg-check` reports 35/35 fields match after the
full fix series.

**Where verification ran:** all gates ran in the main r9 checkout (no worktree —
this two-repo phase commits fixes directly in the owning repo per the
orchestrator's project context; the r9 gates need the nix dev shell, qemu, and
the warm `target/`). Host: `nix develop -c cargo test -p port -p xtask`
(45 passed, 0 failed) and `nix develop -c cargo check -p aarch64` after every
fix. Boot gates (per-fix targets listed per finding below; final sweep after the
last fix): virtexec PASS, virtmulti PASS, virtnotes PASS, virt PASS, virtpanic
PASS.

## Fixed Issues

### CR-01: Demand pager silently corrupts data when bss faults precede data on the shared last data page

**Files modified:** `../r9/aarch64/src/proc.rs`
**Commit:** 8c4f1cb
**Applied fix:** Classify by page at the data/bss boundary, per the review's
prescription. `Segments::classify` now computes a page-granular data
classification end — `data.end.next_multiple_of(PAGE_SIZE_4K)` clamped to never
exceed `bss_end` (empty/tiny bss extends nothing) nor underflow data's own end —
so the shared last-data/head-of-bss page always classifies as `Data`. The
existing `page_in_from_image` zero-fill + byte-exact `content_end` clamp (04-06
Rule 1) then yields exactly the right bytes regardless of fault order: file
bytes below `data_end`, zeros above. The recorded `data`/`bss` ranges stay
byte-exact, so `validate_user_range` is unchanged (the review's constraint).
The stale "Ranges are page-rounded at exec time" comment was corrected (only
text is rounded; the data/bss boundary handling is now documented on
`classify`).
**Verification:** `cargo check -p aarch64`; `boot-test virtexec` PASS (hello +
smokefork + smokenote note-ok + all-exited). *Note:* the first post-fix
virtexec run hit the pre-existing `note-timeout` flake (see "Observations"
below); reruns pass, and the fix is provably inert for the shipped binaries'
page-in behavior (their `0x21000` page classifies Bss before and after).

### WR-01: `map_to` error path abandons the recursive slot-511 restore

**Files modified:** `../r9/aarch64/src/vm.rs`
**Commit:** 8e52bfe
**Applied fix:** Restructured exactly as the review prescribed: the table walk +
destination-entry write now run inside a closure,
`write_recursive_entry(pgtype, old_recursive_entry)` executes unconditionally
after it, and the closure's `Result` is returned — the recursive slot-511
restore happens on every exit, so a mid-walk failure (EntryIsNotTable /
pagealloc) can no longer leave the active table's slot pointing at the other
root and perpetuate the corruption through later `map_to` calls.
**Verification:** `cargo check -p aarch64`; `boot-test virtexec` PASS.

### WR-02: rfork_clone / new_user_proc error paths leave a runnable null-frame child and leak pages

**Files modified:** `../r9/aarch64/src/proc.rs`
**Commit:** 28defb1
**Applied fix:** Both halves. (1) Split the fallible post-construction body of
`rfork_clone` (eager page copy, segment/source mirror, fd dup, forkchild frame)
into `rfork_copy`; on any `Err`, the unwind marks the child `Moribund` (the
scheduler never picks it — the null-frame landmine is disarmed), deallocates
the already-copied user pages recorded in `child.mapping_log`, and frees the
root-table page + kstack pages via `free_root_table_and_kstack` (kernel-side
KZERO+PA aliases stay mapped, matching the existing no-teardown convention).
(2) `new_user_proc`'s table-full path now drops the PROC_TABLE guard first
(the module locking discipline never holds it across a pagealloc call), then
frees the allocated root-table page and kstack pages before returning
`"proc: table full"`.
**Verification:** `cargo check -p aarch64`; `boot-test virtmulti` PASS. *Logic
error class per REVIEW.md — the unwind paths are not reachable from the smoke
profiles (they need OOM/table-full); the construction follows
`free_root_table_and_kstack`'s recorded-PA discipline, but a human should
confirm the unwind ordering before the phase's verification sign-off.*

### WR-03: RFNOWAIT child exit corrupts the parent's AWAIT pivot forever

**Files modified:** `../r9/aarch64/src/proc.rs`
**Commit:** cc7e7d2
**Applied fix:** The review's preferred (Plan 9-matching) arm:
`rfork_clone` now increments `parent.children_live` only when `!nowait` — an
RFNOWAIT child posts no wait record and is invisible to AWAIT, so it no longer
leaves the pivot elevated (which previously hung the parent's next AWAIT in
`sched()` with nothing left to wake it). The `children_live` field comment now
records the exclusion. This makes the documented "RFNOWAIT is honored" claim
(syscall.rs:535-536, HowItWorks.md) actually true.
**Verification:** `cargo check -p aarch64`; `boot-test virtmulti` PASS.

### WR-04: NOTED's NCONT/NRSTR restores `psr` from user-writable memory without sanitizing to EL0t

**Files modified:** `../r9/aarch64/src/trap.rs`, `../r9/aarch64/src/syscall.rs`
**Commit:** 378f46c
**Applied fix:** Added `TrapFrame::force_psr_el0t()` (`self.psr &= !0xf`, the
review's exact mask) with a comment documenting the trust-boundary rationale;
`sys_noted`'s NCONT/NRSTR arm calls it immediately after the 280-byte
`copy_nonoverlapping` restore. A handler-edited M field can no longer steer
trap.S's `msr spsr_el1` + `eret` to EL1. The saved `type` field is left as-is —
aarch64/ABI.md already documents it informational-only, per the review. The
frozen layout is untouched (a method, not a field).
**Verification:** `cargo check -p aarch64`; `cargo xtask ureg-check` 35/35;
`boot-test virtnotes` PASS.

### WR-05: deliver_notes has no in-handler guard — a second note overwrites `note_ureg` mid-handler

**Files modified:** `../r9/aarch64/src/trap.rs`
**Commit:** 10ba502
**Applied fix:** The review's donor-semantics guard: `deliver_notes` returns
early when `proc.note_ureg != 0` (note_ureg doubles as the donor's
`up->notified`), leaving the note queued; `noted()` clears `note_ureg` and the
next return-to-EL0 boundary retries delivery. No new queue machinery.
`deliver_notes`' doc comment now documents the guard.
**Verification:** `cargo check -p aarch64`; `boot-test virtnotes` PASS.

### WR-06: fault-kill paths bypass the 04-07 lifecycle — parent AWAIT hangs, kernel wedges with IRQs masked

**Files modified:** `../r9/aarch64/src/fault.rs`
**Commit:** 3906127
**Applied fix:** `kill_current_process` now routes through
`syscall::exit_tail(proc, b"fault")` — posts the wait record, wakes an
AWAITing parent, frees user mappings + root-table page + kstack by the
recorded PAs, and hands off to `sched_from_moribund` — exactly the shape
deliver_notes's no-handler kill already used. The IRQs-masked `wfi` wedge is
gone (one wild pointer kills one process now); the all-exited clean halt
remains as exit_tail's tail; the no-current-process pre-userspace case still
parks. The stale single-process-era comment was rewritten. trap()'s
other-EL0-origin sync kill (the cited second site) calls the same function and
is fixed with it. The now-unused `ProcState` import was dropped.
**Verification:** `cargo check -p aarch64`; `boot-test virtexec` PASS +
`boot-test virtmulti` PASS. *Logic-adjacent (lifecycle path): the kill path is
not exercised by the smokes (no binary faults); it reuses the exact
exit_tail/sched_from_moribund shape that EXITS and the note kills already run
under the gates — human confirmation recommended per the report's own
classification.*

### WR-07: exec_replace's "seed can no longer fail" invariant is false — post-free failure strands the proc

**Files modified:** `../r9/aarch64/src/exec.rs`
**Commit:** 63921d4
**Applied fix:** The review's option (b): the post-free `install_image` call in
`exec_replace` now diverges loudly on failure — prints a marker and kills via
`syscall::exit_tail(proc, msg)` (the WR-06 fault-kill shape) instead of
returning -1 to a trap frame whose pc/sp name freed text pages. All three
sites of the false invariant were corrected: exec_replace's doc comment, the
pre-free check-site comment, and `check_seed_fit`'s doc comment (arg-shape
failures still return -1 with the old image intact; only post-free allocation
failures kill).
**Verification:** `cargo check -p aarch64`; `boot-test virtexec` PASS
(exec_replace runs on every child exec in the profile).

### WR-08: user fault on a top-half VA panics the kernel (user-reachable DoS)

**Files modified:** `../r9/aarch64/src/fault.rs`
**Commit:** 54e8c10
**Applied fix:** The `va >= KZERO` guard now falls through to
`unresolvable(frame, class, far, access)` — since WR-06, that kills only the
faulting process via exit_tail. The comment explains the correction: this
handler is routed only lower-EL aborts (EC 0x20/0x24), so a top-half FAR is
always a userland wild pointer; a handler-side kernel-context touch arrives as
a current-EL abort (EC 0x21/0x25) and hits trap()'s dump-and-halt, which is
the real T-04-09 guard. The module-header invariant comment was updated to
match.
**Verification:** `cargo check -p aarch64`; `boot-test virtexec` PASS;
`boot-test virtpanic` PASS (the deliberate-panic profile is unaffected).

## Skipped Issues

None — all in-scope findings were fixed.

## Observations (not findings; for the orchestrator/user)

**Pre-existing `note-timeout` flake in the notes smoke.** During this run, the
boot profiles intermittently failed with smokenote exiting `note-timeout`
(after a successful `smokenote: got note smoke`): observed once on virtexec and
once on virtmulti with fixes applied. A/B testing showed the identical failure
signature reproduces on the **pre-fix base commit b4f858a** (1 of 3 virtmulti
runs), while the fully-fixed main went 3/3 on the final virtmulti sweep — the
flake is pre-existing, not introduced or measurably worsened by these fixes.
All nine fixes are provably inert for the smoke binaries' flows (error paths,
RFNOWAIT, high-FAR, multi-note delivery — none exercised; CR-01's shared-page
classification does not trigger for the shipped layouts: the `0x21000` page
classifies Bss before and after). The flake mechanism appears to be a timing
race in the note-delivery/resume path when the note is delivered at the
`notify()` return boundary (before main's first `seen()` call faults the bss
page in) — consistent with init.c's documented "no rendezvous primitive this
era" CPU-spin ordering. Recommend a follow-up investigation (candidate phase-5
debug item) rather than expanding this fix run's scope.

---

_Fixed: 2026-11-02T20:06:51Z_
_Fixer: the agent (gsd-code-fixer)_
_Iteration: 1_
