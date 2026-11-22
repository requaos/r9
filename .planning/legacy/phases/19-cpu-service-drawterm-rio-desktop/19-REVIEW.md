---
phase: 19-cpu-service-drawterm-rio-desktop
reviewed: 2026-11-17T00:00:00Z
depth: standard
files_reviewed: 51
files_reviewed_list:
  - ../r9/flake.nix
  - ../r9/port/src/dat.rs
  - ../r9/port/src/devip/proto.rs
  - ../r9/port/src/devip/stack.rs
  - ../r9/port/src/devip_tests.rs
  - ../r9/port/src/devssl.rs
  - ../r9/port/src/devssl_tests.rs
  - ../r9/port/src/lib.rs
  - ../r9/port/src/namec.rs
  - ../r9/port/src/rc4.rs
  - ../r9/port/src/namec_tests.rs
  - ../r9/port/tests/namespace_test.rs
  - ../r9/port/src/fdgrp.rs
  - ../r9/port/src/devmnt.rs
  - ../r9/port/src/pgrp.rs
  - ../r9/x86_64/lib/config_q35cpu.toml
  - ../r9/x86_64/lib/grub.cfg
  - ../r9/x86_64/src/exec.rs
  - ../r9/x86_64/src/main.rs
  - ../r9/x86_64/src/platform/q35/mb2.rs
  - ../r9/x86_64/src/platform/q35/timer.rs
  - ../r9/x86_64/src/syscall.rs
  - ../r9/xtask/src/config.rs
  - ../r9/xtask/src/lib.rs
  - ../r9/xtask/src/main.rs
  - authdomain.local.EXAMPLE
  - build/x86_64/README.md
  - build/x86_64/authdomain-gen.c
  - build/x86_64/authdomain-selftest.sh
  - build/x86_64/authdomain.example
  - build/x86_64/cpu-bringup.expect
  - build/x86_64/drawterm-gate.expect
  - build/x86_64/mkimage.sh
  - build/x86_64/smoke-r9.expect
  - build/x86_64/stage-build.sh
  - build/x86_64/subset-build.sh
  - build/x86_64/root-tree/bin/service/tcp17010
  - build/x86_64/root-tree/rc/bin/cpurc
  - build/x86_64/root-tree/usr/glenda/lib/profile
  - build/x86_64/tiers/tier2-services.txt
  - flake.lock
  - flake.nix
  - nix/boot-r9-x64.sh
  - nix/drawterm-gate-x64.sh
  - nix/drawterm-x64.sh
  - nix/drawterm.nix
  - nix/smoke-r9-x64.sh
  - sys/src/cmd/unix/drawterm/Make.config
  - sys/src/cmd/unix/drawterm/cpu.c
  - sys/src/cmd/unix/drawterm/drawterm.h
  - sys/src/cmd/unix/drawterm/gui-x11/x11.c
findings:
  critical: 1
  warning: 2
  info: 2
  total: 5
  necessity: 1
status: issues_found
---

# Phase 19: Code Review Report

**Reviewed:** 2026-11-17
**Depth:** standard
**Files Reviewed:** 51 (harvey cwd + the sibling ../r9 repo)
**Status:** issues_found

## Summary

Standard-depth adversarial review of the phase-19 surface: the r9 kernel
half (devssl `#D` + rc4/sha1, devmnt tag-mux and zero-length-write
semantics, mount-identity keying, close-on-exec, fd/pgrp teardown drains,
cmdline-to-env bootconf seed, `#!` exec, syscall arms) and the harvey half
(the §15 cpu bake, authdomain-gen secret derivation and creds emission, the
drawterm wrapper/-O/-G/-Shift client surface, the nix gates, and the
vendored drawterm).

The overall quality is high: the 9P client treats every server reply as
untrusted input with exact-consume cursor checks; the mb2/cmdline parser is
bounds-checked and exhaustively pinned; the authdomain pairing has
generate-time decrypt round-trip teeth plus doctored-input negative
controls; the gates redact the per-bake DEMO password from retained
captures; and no real password or key material landed in any committed file
(all committed templates carry CHANGE-ME placeholders only; flake.lock is
hash-only pin data). The donor-fidelity discipline (D-06) was verified
against the donor sources for devssl.c and rc4.c framing/secret semantics.

Five findings: one BLOCKER (a one-byte stack out-of-bounds write in
authdomain-gen's verify/info paths when a staged manifest is exactly the
file cap), two Warnings (modulo bias in generated passwords; a stale-offset
merge in `FdGroup::op_done` on a shared-fd-group close/open race that the
code's own comments contradict), and two Info items (no zeroization of
devssl secrets/rc4 state at conv free — donor parity; a donor-carried
impossible-state USTOP guard).

Coverage note: all security/correctness-critical sources were read in full
(devssl.rs, rc4.rs, devmnt.rs, namec.rs, fdgrp.rs, pgrp.rs, dat.rs,
devip/proto.rs, devip/stack.rs, syscall.rs, exec.rs, mb2.rs, main.rs,
timer.rs, authdomain-gen.c, mkimage.sh, the four gates/drivers, the
drawterm client files). The three largest test files
(`../r9/port/src/devip_tests.rs`, `namec_tests.rs`,
`tests/namespace_test.rs`) and `../r9/xtask/src/main.rs` were reviewed via
full reads of their phase-relevant sections plus structural/anti-pattern
passes over the remainder; no issues affecting test reliability were found.
`necessity-review.md` was read and applied; no agent skills are configured
for this agent type.

## Critical Issues

### CR-01: One-byte stack out-of-bounds write in `verify()`/`info()` when a staged file is exactly Filecap bytes

**File:** `build/x86_64/authdomain-gen.c:689` (verify) and `build/x86_64/authdomain-gen.c:849` (info)
**Issue:** `readfile()` is designed to accept a file of exactly `cap`
bytes: at `got == cap` it performs one extra 1-byte read and only errors if
that read returns data (lines 259-267). Both `verify()` and `info()` then do
`n = readfile(dir, "manifest", (uchar*)manifest, Filecap); manifest[n] = 0;`
where `manifest` is `char manifest[Filecap]` (65536 bytes). A manifest file
of exactly 65536 bytes — precisely the class of doctored input the tool's
own negative-control teeth (`AUTHBAKE_DOCTOR`, the self-test's four
doctored classes) exist to catch — yields `n == Filecap` and writes
`manifest[65536] = 0`, one NUL byte past the end of a 64 KiB stack buffer.
This is a memory-safety defect on the untrusted-input path of a host tool
invoked at every bake and every `-V` verification.
**Fix:** Size the buffer one byte larger than the cap (and keep the
readfile cap at Filecap), or reject `n == cap` outright:

```c
char manifest[Filecap+1];
n = readfile(dir, "manifest", (uchar*)manifest, Filecap);
manifest[n] = 0;	/* n <= Filecap always fits */
```
Apply the same shape in `info()` (`manifest[Filecap+1]`), or add
`if(n == Filecap) sysfatal("manifest too long");` in both arms.

## Warnings

### WR-01: Modulo bias in generated passwords (`alph[r[i] % 62]`)

**File:** `build/x86_64/authdomain-gen.c:234`
**Issue:** `genpassword()` maps raw `/dev/urandom` bytes onto the 62-symbol
alphabet with `r[i] % (nelem(alph)-1)`. 256 is not a multiple of 62
(256 = 4·62 + 8), so the first 8 symbols ('0'-'7') are each selected with
probability 5/256 while the other 54 get 4/256 — a 25% relative
favoritism. The entropy loss is small (~0.02 bits/char), but these
passwords are real key material whenever an operator relies on the
generated values (machine_password derives the nvram machkey — the
/adm/keys master key; user passwords derive p9sk1 keys), and the
rejection-sampling fix is one line.
**Fix:**

```c
do {
	randbytes(&c, 1);
} while(c >= 256 / (nelem(alph)-1) * (nelem(alph)-1));	/* 248: uniform floor */
out[i] = alph[c % (nelem(alph)-1)];
```

### WR-02: `FdGroup::op_done` merges the streaming offset into a possibly REFILLED slot — the comments' premise contradicts the op_ref design

**File:** `../r9/port/src/fdgrp.rs:120-125`
**Issue:** `op_done(fd, c)` writes `slot.offset = c.offset` into whatever
chan currently occupies `fd`. The comment covers only the "slot emptied
mid-op" case ("a close raced us — its close-dispatch already ran"), but a
sibling in a SHARED fd group can also close(fd) and then open() a
different file into the same slot while the original op is blocked in
pread/pwrite — exactly the interleave `op_ref` was built for (its own doc:
"libthread's whole shape: aan's blocked reader proc and writer proc ride
one netfd"). On wake, the blocked op's advanced offset is merged into the
unrelated new chan, silently jumping its streaming position. Note that
`store()`'s comment ("impossible this era because the only blocking op
(pipe) has no shared-table caller") is contradicted by `op_ref`'s own
documentation, so the guard-free merge is not actually covered by an
impossibility argument.
**Fix:** Merge the offset only when the slot still names the SAME file
identity, e.g. compare `(dev, devno, qid.path)` of the op chan and the slot
chan (or `Arc::ptr_eq` on `mnt_fid` for DEV_MNT chans) before the
assignment; otherwise just `drop_op_ref` and discard the offset.

## Info

### IN-01: devssl secrets and RC4 state are freed without zeroization — key material persists in kernel heap

**File:** `../r9/port/src/devssl.rs:160-176` (OneWay), `../r9/port/src/devssl.rs:1019-1043` (close)
**Issue:** On the last close, the conv's `Vec<u8>` secrets and the
`Rc4state` permutation are dropped into the kernel heap without being
zeroed first, so the p9sk1 session keys (the dec64'd mksecret material)
remain readable in freed kernel memory until the pages are reused. The
donor has the same behavior (`free(s->in.secret)` etc. in sslclose, no
memset), so this is donor parity, and the hardening milestone owns crypto
upgrades — recorded here because the phase's security surface explicitly
includes devssl key handling.
**Fix:** When the hardening milestone lands, zero `OneWay::secret` and
`Rc4state::state` on direction drop (a `Drop` impl or an explicit
`secret.iter_mut().for_each(|b| *b = 0)` in `close()`'s last-ref arm).

### IN-02: Impossible-state guard — `check_data_bss_below_ustop` cannot fire for any parseable header

**File:** `../r9/x86_64/src/exec.rs:728-733`
**Category:** necessity
**Issue:** The guarded state (u32-derived text/data/bss reaching USTOP at
~128 TiB) cannot occur: data_va ≤ ~66 MiB and data+bss ≤ ~8.6 GiB for
maximal u32 fields, far below `USTOP`. The module's own comment admits this
("this can never fire for a parseable header — it is defense-in-depth"),
and the boundary arithmetic is host-pinned deliberately. It rests on
donor-fidelity judgment (the donor sysexec carries the same check for
formats/arches r9 does not have), so this is Info, not a removal demand.
**Fix:** Leave as-is for donor fidelity (the comment already documents the
invariant); if the donor-fidelity stance ever relaxes, retire the check to
the a.out-format doc.

---

_Reviewed: 2026-11-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
