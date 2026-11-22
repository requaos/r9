---
phase: 05-shell-boot-one-command-integration
reviewed: 2026-11-03T00:00:00Z
depth: standard
files_reviewed: 71
files_reviewed_list:
  - build/aarch64/drive-r9.expect
  - build/aarch64/init.c
  - build/aarch64/mkimage.sh
  - build/aarch64/pidwait.c
  - build/aarch64/run-r9.sh
  - build/aarch64/smoke-r9.expect
  - build/aarch64/stacksmoke.c
  - build/aarch64/stage-build.sh
  - build/aarch64/subset-build.sh
  - build/aarch64/trace-boot.rc
  - build/aarch64/trace-sweep.rc
  - build/aarch64/trace-syscalls.py
  - build/REPRODUCIBILITY.md
  - flake.lock
  - flake.nix
  - .github/workflows/r9-shell.yml
  - .gitignore
  - nix/boot-r9.sh
  - nix/smoke-r9.sh
  - ../r9/aarch64/Cargo.toml
  - ../r9/aarch64/lib/config_virtshell.toml
  - ../r9/aarch64/src/exec.rs
  - ../r9/aarch64/src/fault.rs
  - ../r9/aarch64/src/main.rs
  - ../r9/aarch64/src/platform/virt/pl011.rs
  - ../r9/aarch64/src/platform/virt/timer.rs
  - ../r9/aarch64/src/proc.rs
  - ../r9/aarch64/src/syscall.rs
  - ../r9/aarch64/src/trap.rs
  - ../r9/aarch64/src/trap.S
  - ../r9/flake.nix
  - ../r9/.gitignore
  - ../r9/HowItWorks.md
  - ../r9/port/src/allocator.rs
  - ../r9/port/src/dat.rs
  - ../r9/port/src/devcons.rs
  - ../r9/port/src/devdup.rs
  - ../r9/port/src/devenv.rs
  - ../r9/port/src/devmnt.rs
  - ../r9/port/src/devpipe.rs
  - ../r9/port/src/devproc.rs
  - ../r9/port/src/devroot.rs
  - ../r9/port/src/devsrv.rs
  - ../r9/port/src/fdgrp.rs
  - ../r9/port/src/lib.rs
  - ../r9/port/src/namec.rs
  - ../r9/port/src/pgrp.rs
  - ../r9/port/tests/05-04-red-evidence.json
  - ../r9/port/tests/05-05-red-evidence.json
  - ../r9/port/tests/05-06-red-evidence.json
  - ../r9/port/tests/05-07-red-evidence.json
  - ../r9/port/tests/05-09-red-evidence.json
  - ../r9/port/tests/devcons_test.rs
  - ../r9/port/tests/devdup_devsrv_test.rs
  - ../r9/port/tests/devenv_test.rs
  - ../r9/port/tests/devmnt_test.rs
  - ../r9/port/tests/devpipe_test.rs
  - ../r9/port/tests/devproc_test.rs
  - ../r9/port/tests/devroot_rawimg_test.rs
  - ../r9/port/tests/fdgroup_test.rs
  - ../r9/port/tests/namespace_test.rs
  - ../r9/README.md
  - ../r9/xtask/src/main.rs
  - ../r9/xtask/src/uregcheck.rs
  - sys/src/9k/boot/bootr9.c
  - sys/src/9k/boot/mkfile.aarch64
  - build/aarch64/root-tree/lib/namespace
  - build/aarch64/root-tree/rc/bin/termrc
  - build/aarch64/root-tree/usr/glenda/lib/profile
  - build/aarch64/trace-namespace
  - build/aarch64/trace-termrc
findings:
  critical: 2
  warning: 8
  info: 13
  total: 23
status: issues_found
---

# Phase 05: Code Review Report

**Reviewed:** 2026-11-03T00:00:00Z
**Depth:** standard
**Files Reviewed:** 71 (both repos: Harvey build/nix/CI + r9 aarch64/port kernel)
**Status:** issues_found

## Summary

The phase-05 surface is unusually well-commented and the gated chain is evidently
green, but the review found two kernel defects that survive precisely because the
gate never exercises them: six of the seven stat-serving devices emit malformed
stat(5) wire records (three strings, no muid — libc's `statcheck` rejects the whole
listing and `dirstat` returns a Dir with **uninitialized** string pointers), and
`sys_stat`/`sys_fstat` are the only buffer-writing syscall handlers that skip the
`ensure_user_resident` cold-buffer step their own T-05-06 rule mandates (a cold
heap page aborts at EL1h and halts the kernel). The namespace/pgrp lock is held
across blocking devmnt RPCs in most namespace syscalls, a latent total-system
deadlock the moment any 9P server takes the pgrp lock while serving. The shell
tooling is otherwise solid; the pkill teardown patterns are too broad (they kill
foreign qemu processes), and the consctl parser silently ignores the natural
`echo rawon >#c/consctl` form.

## Critical Issues

### CR-01: Six devices serve malformed stat(5) records (no muid string)

**Files:** `../r9/port/src/devroot.rs:229-255`, `../r9/port/src/devcons.rs:309-329`,
`../r9/port/src/devenv.rs:455-482`, `../r9/port/src/devpipe.rs:409-435`,
`../r9/port/src/devdup.rs:221-247`, `../r9/port/src/devsrv.rs:271-297`

**Issue:** The stat(5) wire record is 41 fixed bytes followed by **four**
length-prefixed strings (name, uid, gid, muid). All six devices above write only
three (`name`, `"glenda"`, `"glenda"`) and size the buffer accordingly
(`41 + (2+name) + 2*(2+6)`). `../r9/port/src/devproc.rs:269-301` writes four and
its comment even documents the failure mode ("a three-string record fails the
whole directory read ... convM2D.c"), so the requirement was known; the other
devices were never updated. Consequences against Harvey's own libc
(`sys/src/libc/9sys/convM2D.c`):

- `dirread`/`dirreadall` (`sys/src/libc/9sys/dirread.c:23`) runs `statcheck` per
  record; statcheck walks four strings and rejects a 3-string record, failing the
  ENTIRE directory read (`ls /env`, `ls /fd`, `ls /srv`, `ls /dev`, any dirread of
  `#|`/devroot roots).
- `dirstat` (`sys/src/libc/9sys/dirstat.c:30`) calls `convM2D` and **ignores its
  return**; convM2D returns 0 at the missing fourth string *before* assigning
  `d->name/uid/gid/muid`, so `dirstat` returns a `Dir` whose string fields are
  uninitialized malloc garbage — downstream callers print or dereference garbage
  pointers. This is worse than a clean failure.

The malformation is latent in the gated flows (ps reads /proc — the one correct
device; ls /bin rides devmnt → the real paqfs server writes proper records), which
is why the smoke passes. Worse, the host tests PIN the wrong shape:
`../r9/port/tests/devenv_test.rs:294` and `devroot_rawimg_test.rs:239` assert
`n == 41 + (2+name_len) + 2*(2+6)` with a comment mislabeling it "convD2M" — the
suite encodes the defect. Only `devproc_test.rs:66-95` walks four strings.

**Fix:** append the fourth string in every device writer (and the `need`
arithmetic), e.g. in each `write_stat_record`:

```rust
w.string(name);
w.string(b"glenda");
w.string(b"glenda");
w.string(b"glenda"); // muid — the fourth string statcheck walks
```

and change `need` to `41 + (2 + name.len()) + 3 * (2 + 6)` (likewise the
`listing.resize(...)` calls that pre-size directory buffers in devcons/devenv/
devpipe/devdup/devsrv `read` arms). Then fix the two tests that pin the 3-string
byte count, and add a statcheck-shaped (4-string) round-trip assertion per device
so the wire shape is pinned test-side like devproc's.

### CR-02: sys_stat/sys_fstat skip the cold-buffer residency step — user-reachable kernel halt

**File:** `../r9/aarch64/src/syscall.rs:844-897`

**Issue:** Every other syscall handler that writes a user buffer forces the target
resident before the kernel touches it: `sys_pread` (:372), `sys_pipe` (:1078),
`sys_fd2path` (:1258), `sys_errstr` (:1437), `sys_await` (:1534), `sys_seek`
(:1391), `sys_fversion` (:723). `sys_stat` (:853-867) and `sys_fstat` (:884-891)
only `validate_user_range` (a segment-membership check) and then write through
`user_slice_mut` into `namec::stat_chan`. A page that is in-segment but never
demand-faulted is unmapped; the kernel write takes an EL1h data abort (EC 0x25),
which trap.rs does NOT route to fault::user_fault — it lands in the
dump-and-halt arm (`Unhandled interrupt` → `kernel halted`, trap.rs:476-488).
That is a full kernel halt on a legal user syscall — the exact "cold-buffer class"
05-06 documented and fixed everywhere else ("an in-segment page that was never
demand-faulted would take an EL1h abort mid-delivery — a kernel panic, never
acceptable on a user-triggerable path", fault.rs:186-189).

Reachability is ordinary, not exotic: bss/data are eagerly mapped at exec, but
brk_-grown heap pages are demand-paged. libc `dirstat` stat buffers are
`malloc(sizeof(Dir) + BIT16SZ + nd)` (~640 bytes); a malloc block that straddles
into a not-yet-touched heap page (normal heap fragmentation) suffices. rc's own
`dirstat`/`stat` calls (`~`, `cd` existence probes, etc.) can hit this.

**Fix:** add the same line the siblings carry, immediately after the
`validate_user_range` in both handlers:

```rust
fault::ensure_user_resident(proc, buf_va as usize, (buf_va + n) as usize);
```

## Warnings

### WR-01: consctl parser silently ignores newline-terminated words

**File:** `../r9/port/src/devcons.rs:483-505`

**Issue:** the QID_CONSCTL write path splits the buffer only on `b' '` and compares
whole words against `b"rawon"`/`b"rawoff"`. The natural invocation —
`echo rawon >#c/consctl` — writes `"rawon\n"`, which splits to `["rawon\n"]`,
matches nothing, and returns `Ok(buf.len())`: the mode switch is silently
discarded with a success status. The donor's conswrite tokenizes via parsecmd
(whitespace including `\n`).

**Fix:** split on all whitespace, e.g. `for w in buf.split(|&b| b == b' ' || b == b'\n' || b == b'\t')`
(ignoring empty tokens), or `buf.split_ascii_whitespace()`.

### WR-02: sys_sleep wraps negative arguments into ~49-day sleeps

**File:** `../r9/aarch64/src/syscall.rs:1471-1491`

**Issue:** `let ms = args[0] & 0xffff_ffff` treats the slot as an unsigned 32-bit
value. Plan 9 `sleep(long)` is signed; the donor's syssleep yields when `n <= 0`.
Here `sleep(-1)` arrives as `0xffffffff` and blocks the proc for 4294967295 ticks
(≈49.7 days at 1 kHz) with no wake path other than expiry (the handler documents
that notes don't interrupt sleep this era). Any userland passing a negative
(computed) duration wedges itself.

**Fix:** interpret as signed: `let ms = args[0] & 0xffff_ffff; if (ms as i32) <= 0 { proc::preempt(); return 0; }` (keeping the existing zero-yield arm).

### WR-03: cooked-mode ^D on an empty line never yields EOF

**File:** `../r9/port/src/devcons.rs:707-716`

**Issue:** on CTLD the in-assembly line is moved to `lines` — but when the line is
empty, `lines.extend([])` appends nothing, so no completed line ever becomes
readable and the reader blocks forever. The donor's ^D sends the accumulated line
*without* the byte, and an empty accumulated line produces a zero-length read —
the EOF that lets `rc` exit on ^D at a fresh prompt. As written, ^D at the prompt
hangs the shell read; only the typed `exit` works (which is why the gate missed
it).

**Fix:** track the ^D case so an empty line still completes a (zero-length) line:
e.g. push a marker/flag that lets the reader return `Ok(0)` once, per the donor's
send-without-append arm.

### WR-04: pgrp lock held across blocking devmnt RPCs in most namespace syscalls

**File:** `../r9/aarch64/src/syscall.rs:558-571 (open), 606-623 (bind), 639-654
(chdir), 672-697 (unmount), 856-866 (stat), 985-996 (exec's namec), 1181-1237
(create)`

**Issue:** these handlers hold `pg.lock(&node)` (the pgrp MCS spinlock) across
`namec::namec`, and `sys_open` also across `namec::open_chan`. For paths that
cross a mount point, namec drives devmnt Twalk RPCs and open_chan drives Topen —
both block mid-syscall on the pipe (scheduling the server proc) while the lock is
held. `sys_mount` (:810-812) and `sys_exec`'s open/stage (:997-998) explicitly
document and follow the opposite discipline ("NO pgrp lock is held across the
RPC"). The chain's own paqfs server was forked with `fork()` and therefore SHARES
the client's pgrp (neither RFNAMEG nor RFCNAMEG); if any 9P server proc ever takes
that lock while serving (an open/stat/bind inside the server), it spins on the
MCS lock in IRQ-masked context — never preempted, never released: a total-system
deadlock. Today's measured paqfs serving loop never touches pgrp, so the gate is
green, but the lock-ordering tripwire is built into the shipped design.

**Fix:** restructure as sys_mount does: take the lock for the mount-table consult
only, drop it before any device op that can RPC (or split namec into
resolve-under-lock + reopenable device phase), and apply uniformly across the
seven handlers.

### WR-05: new_user_proc leaks the root-table and kstack pages on mid-construction failure

**File:** `../r9/aarch64/src/proc.rs:763-790`

**Issue:** in the kstack construction loop, the VA-contiguity check (:771) and a
kstack map failure (:787) `return Err(...)` with the root-table page (allocated at
:734) and all previously allocated/mapped kstack pages still live. The WR-02
unwind (:797-815) covers only the table-full failure after that point. Kernel
pages leak per failed rfork/exec attempt.

**Fix:** funnel the early returns through a common unwind that deallocates
`root_pa` and `kstack_pas[..i]` (the same loop the table-full arm runs).

### WR-06: exec accepts truncated binaries (zero-filled text tail) and devmnt read_all rejects an exact-cap file

**File:** `../r9/aarch64/src/exec.rs:83-114, 272-321`; `../r9/port/src/devmnt.rs:830-843`

**Issue (a):** `parse_aout`/`read_aout` read only the 40-byte header and never
check that the header's `text`/`data` sizes fit within the actual file length.
A truncated binary execs "successfully": the pager's `read_at` clamps at EOF and
`page_in_from_image` zero-fills first, so the missing tail is zeros — the process
then executes zeroed text and dies later in a misleading place. The donor's
sysexec fails the exec on a short segment read.

**Issue (b):** `read_all` checks `if out.len() >= cap { Err }` at the TOP of the
loop, so a file of exactly `EXEC_STAGE_CAP` bytes errors ("file too large")
without ever attempting the final zero-length EOF read that would have
terminated the loop cleanly.

**Fix:** (a) after parsing, compare `HDR_SIZE + text + data (+ syms, per the
image's length semantics)` against the backing's length (PaqDir.length /
staged Vec length) and Err "exec: truncated binary" up front. (b) move the cap
check after the read so an exact-fit file terminates via the empty-read EOF arm.

### WR-07: pkill teardown patterns kill foreign r9 qemu processes

**File:** `build/aarch64/drive-r9.expect:71`, `nix/smoke-r9.sh:137`

**Issue:** the survivor-sweep patterns (`[q]emu-system-aarch64.*aarch64-unknown-none-elf`
and `[q]emu-system-aarch64.*kernel-virt\.elf`) match by kernel-name class, not by
this run's identity. A developer's concurrent interactive `nix run .#boot-r9`
(which uses the same `kernel-virt.elf`) — or a parallel CI job on a shared runner —
is killed by the gate's cleanup. smoke-r9.sh's own comment acknowledges the class
("a dev's interactive boot-r9 uses the same kernel, but never concurrently with
the gate"), which is an assumption, not a guard.

**Fix:** key the sweep on a per-run unique value — e.g. record the spawned qemu
pid/pgid at launch (run-r9.sh already tracks QEMU_PID; the smoke driver has
`spawn_pid`) and pkill on the run's unique capture-path/pidfile, or pass
`-name "r9smoke-$$"` into qemu's argv and match on that.

### WR-08: devroot directories are unreadable

**File:** `../r9/port/src/devroot.rs:201-205`

**Issue:** `DevRoot::read` errors on any directory ("devroot: read of a
directory"). The donor's devroot serves directory listings (devdirread over the
root tree). Any dirreadall of a devroot directory — e.g. reading `/` in the
window before newns binds the mounted union over it, or a `walk`+`read` pair from
a diagnostic tool — fails outright. Latent in the gated chain (post-mount listings
ride devmnt), but it is a hole in the claimed donor fidelity, and CR-01's fix will
make listings parseable for the OTHER devices while devroot dirs still can't be
read at all.

**Fix:** serve a stat-wire listing of the PaqDir's children (the paqfs reader
already exposes `read_dir`); reuse the (fixed, 4-string) StatWriter.

## Info

### IN-01: exec arg-count bounds disagree between the syscall layer and the loader

**File:** `../r9/aarch64/src/syscall.rs:943` (MAX_EXEC_ARGS=32) vs
`../r9/aarch64/src/exec.rs:56` (MAX_ARGS=16)

**Issue:** sys_exec admits up to 32 args; exec's `check_seed_fit` rejects >16.
The user-visible error is identical ("too many args"), so this is consistency
only, but the two constants will drift. Pick one (16 fits the seed-page budget
with the 128-byte arg cap) and share it.

### IN-02: HowItWorks.md pins stale constants

**File:** `../r9/HowItWorks.md:64` — "a 16-slot fd table" (now MAX_FDS=32,
proc.rs:67) and "a fixed 8-proc array" (now MAX_PROCS=16, proc.rs:63).

**Fix:** update the prose to the current constants.

### IN-03: /usr/bin/grep hardcoded in the build scripts

**File:** `build/aarch64/subset-build.sh:108,250`, `build/aarch64/stage-build.sh:40,117`,
`build/aarch64/mkimage.sh:129,131`

**Issue:** the comment (mkimage.sh:128) says plan9 grep lacks -q — true, but
`/usr/bin/grep` does not exist on NixOS-style Linux hosts, and the native-linux
leg is a supported platform (subset-build.sh:250 runs on both legs). The devshell
already provides GNU grep on PATH.

**Fix:** resolve a host grep at the top (`GREP=$(command -v /usr/bin/grep || command -v grep)`)
or note the darwin-only assumption at each site.

### IN-04: devsrv fd-number parse wraps on oversized input

**File:** `../r9/port/src/devsrv.rs:208-211`

**Issue:** `fd = fd * 10 + digit` over up to 31 digits wraps silently (release
build) instead of erroring; devproc::walk (:191-194) and devdup's parse_fd
(:91-99) use checked arithmetic. A wrapped value then resolves the wrong fd or
fails confusingly.

**Fix:** use `checked_mul`/`checked_add` like the sibling devices.

### IN-05: trace-syscalls.py leaks a socket per poll iteration

**File:** `build/aarch64/trace-syscalls.py:461-463, 1002-1004`

**Issue:** each pass through the srv9 readiness loop constructs a fresh
`socket.socket(...).connect_ex(...)` and never closes it (up to 60 per run).
Harmless at this scale; `with socket.socket(...) as s: s.connect_ex(...)` closes
deterministically.

### IN-06: CI workflow supply-chain rule applied narrowly; no cache/concurrency controls

**File:** `.github/workflows/r9-shell.yml:35-38`

**Issue:** the header's T-05-24 rule (40-char SHA pins) is applied to
nix-installer-action but `actions/checkout@v4` remains tag-pinned; and the job
has no `concurrency` group and no nix store cache, so every push/PR runs a full
TCG rebuild + smoke serially (90-minute budget each).

**Fix:** pin checkout by SHA too (or document the scoped exception), consider
`concurrency: { group: r9-shell-${{ github.ref }}, cancel-in-progress: true`,
and a nix cache if runtime becomes a problem.

### IN-07: allocator free_misc carve can register a zero-size block

**File:** `../r9/port/src/allocator.rs:410-430`

**Issue:** for a small misc allocation with align > size (e.g. 64 bytes at
4096-align), the carve path advances the block by exactly its size and records
`size = 0` on the misc list — a permanent dead entry. Benign (never matches a
real allocation), but a guard (`if size < offset + MIN_ALLOC_SIZE { leak quietly }`)
would keep the list honest.

### IN-08: formatting glitch in fault.rs

**File:** `../r9/aarch64/src/fault.rs:236` — `fn page_in_zeroed(...) {    let pa = ...`
puts the opening brace and first statement on one line. Cosmetic.

### IN-09: stale MAX_PROCS value in a proc.rs comment

**File:** `../r9/aarch64/src/proc.rs:799` — "reachable with MAX_PROCS = 8" though
MAX_PROCS is 16 (proc.rs:63). Comment-only.

### IN-10: leftover "XXX: Just for testing." markers in shipped devcons code

**File:** `../r9/port/src/devcons.rs:40-41, 61` — `Console::putstr` and `print`
carry `// XXX: Just for testing.` on what is now the kernel's console print path.

### IN-11: expect drivers open the capture after spawn

**File:** `build/aarch64/smoke-r9.expect:76-83`, `build/aarch64/drive-r9.expect:59-61`

**Issue:** `log_file` is set after `spawn`; output emitted in the spawn→log_file
window is matched but not captured. In practice qemu boot output is slow enough
that nothing is lost, but moving `log_file` ahead of `spawn` (expect supports
this) removes the window.

### IN-12: DEL does not wake already-blocked secondary console readers

**File:** `../r9/port/src/devcons.rs:724-757`

**Issue:** when the DEL processor posts "interrupt" to the blocked waiters' note
groups, those waiters are NOT readied (their wake requires new RX bytes via the
tick's cons_rx_pending check); the queued note delivers only after the next
keystroke. The donor interrupts a note-pending qread immediately. Single-reader
rc is unaffected (it IS the DEL processor), so this is an era-shaped delta — but
it is undocumented in the module header's DEL contract ("the reading syscall
fails interrupted" describes only the current reader).

**Fix:** either wake the waiter pids alongside the post (the hooks already carry
them), or document the deferred delivery explicitly.

### IN-13: flake.lock carries two nixpkgs pins

**File:** `flake.lock:3-34` — root nixpkgs at `31ecbaca…`, r9's transitive
nixpkgs at `1c89083e…`. Documented by REPRODUCIBILITY.md (transitive capture is
intended), so this is closure-bloat information only: two nixpkgs checkouts are
fetched/evaluated per clean run.

---

_Reviewed: 2026-11-03T00:00:00Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
