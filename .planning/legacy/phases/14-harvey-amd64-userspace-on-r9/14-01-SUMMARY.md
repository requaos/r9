---
phase: 14-harvey-amd64-userspace-on-r9
plan: 01
subsystem: abi-freeze
tags: [abi, ureg, x86_64, r9, enforcement, documentation]
duration: 21 min
completed: 2026-11-08
requires:
  - "Phase-13-complete r9 HEAD 029e32f (q35 platform, boot-test gates, CR-01-fixed trap path)"
provides:
  - "r9 x86_64 184-byte 9k-verbatim Ureg with always-on const asserts (the D-01 frozen frame every later ring-3 plan builds against)"
  - "amd64/ABI.md — the frozen full exec interface publication (wire contract, stub quirks, syscall table, frame offsets, entry state, note delivery, stack policy, enforcement appendix)"
  - "dual-arch cargo xtask ureg-check with the D-02b negative control, live as a boot-test precondition on BOTH arches"
affects:
  - "../r9/x86_64/src/{dat,trap}.rs (frame layout + producers)"
  - "../r9/xtask/src/{uregcheck,main}.rs (the drift gate)"
  - "every Phase-14 plan that touches ring-3, syscall entry, or exec"
tech-stack:
  added: []
  patterns:
    - "two-layer ABI enforcement + negative control, amd64 re-key of the aarch64 Phase-4 shape"
    - "packed u16 segment quartet stores (9k syscallentry (15*8+n)(SP) arithmetic) in naked asm"
key-files:
  created:
    - amd64/ABI.md
  modified:
    - ../r9/x86_64/src/dat.rs
    - ../r9/x86_64/src/trap.rs
    - ../r9/xtask/src/uregcheck.rs
    - ../r9/xtask/src/main.rs
key-decisions:
  - "D-01 landed: one 184-byte layout, no compat shim; ds/es/fs/gs are u16 packed in one qword at 120, type@128, cs@152"
  - "SWAPPED-witness re-derivation: intrspecial/intrregular's +24 sites are stub-entry-relative (below the error word) and therefore layout-independent — verified unchanged; intrcommon's btr rides UREG_CS_OFFSET (176->152) with an in-place CR-01 comment"
  - "D-02 landed: amd64/ABI.md freezes the FULL interface incl. the nsec no-spill rule, _seek/_exits renames, the loud-unimplemented dispatch rule, and the USTKTOP 0x00007ffffffff000 stack policy as r9's adopted discretion resolution"
  - "D-02b landed: the negative control mutates an in-memory copy of the REAL amd64 header (cs -> cseg) and requires rejection; a pass (or an inapplicable mutation) is the loud NEGATIVE CONTROL FAILED"
requirements-completed: [XEXEC-01]
commits: 1
plan_head_before: 9fd98ea65a346313021378fdde21850ae396f081
plan_head_after: 0aeab50320049c1250daf70af5169ffe2e6991ab
actuals:
  tokens: 11928
  tasks: 3
  commits: 1
status: complete
coverage:
  - deliverable: "184-byte 9k-verbatim Ureg + intrcommon/witness re-key in r9 (Task 1)"
    verification:
      - kind: command
        ref: "cargo xtask boot-test --arch x86-64 --config q35 (x2), q35net, aarch64 virt — all PASS"
        status: pass
      - kind: command
        ref: "cargo xtask check/clippy green; x86_64 bins 26 tests green (host-target leg)"
        status: pass
      - kind: command
        ref: "acceptance greps: ds:u16 present, ds:u64 retired, 184 size assert, consts 16/19 re-keyed, aarch64+port diff empty"
        status: pass
    human_judgment: false
  - deliverable: "amd64/ABI.md frozen publication (Task 2)"
    verification:
      - kind: command
        ref: "acceptance greps: 184 table, 9syscall/sys.h citation, l64syscall.s donor, nsec rule, 0x00007ffffffff000 policy, Sfmask/FMASK, provenance, SP+0 rule — all non-zero"
        status: pass
    human_judgment: false
  - deliverable: "dual-arch ureg-check + negative control + x86_64 boot-test gate (Task 3)"
    verification:
      - kind: command
        ref: "cargo test -p xtask — 97 passed incl. 6 new amd64 tests (quartet-removal drift pin, negative-control leg)"
        status: pass
      - kind: command
        ref: "cargo xtask ureg-check — both arms Match + negative control fired; q35 boot-test green through the gate; R9_HARVEY_DIR=/nonexistent-coverage boot-test exits 1 with the named error before qemu"
        status: pass
    human_judgment: false
---

# Phase 14 Plan 1: Ureg ABI Freeze (184-byte 9k-verbatim layout + amd64/ABI.md + dual-arch enforcement) Summary

Reworked r9's x86_64 Ureg from the non-conforming 208-byte upstream shape to the 184-byte 9k-verbatim layout (u16 segment quartet at 120, type@128, cs@152) with always-on const asserts, re-derived every frame producer and SWAPPED-witness site, published `amd64/ABI.md` as the frozen full exec interface, and extended the ureg-check gate to amd64 with a firing negative control and the x86_64 boot-test precondition — every Phase-13 gate re-green on the reworked frame.

**Duration:** 21 min (2026-11-08T18:44:17Z → 2026-11-08T19:05:22Z)
**Tasks:** 3/3 complete
**Files:** 5 (1 created Harvey-side, 4 modified r9-side)

## Commits

| Repo | Hash | Message |
|------|------|---------|
| r9 | 10fea36 | feat(14-01): rework x86_64 Ureg to the 184-byte 9k-verbatim layout (D-01) |
| harvey | 0aeab5032 | docs(14-01): publish amd64/ABI.md — the frozen full exec interface (XEXEC-01, D-02) |
| r9 | 0f01476 | feat(14-01): ureg-check amd64 arm + negative control + x86_64 boot-test gate (D-02b) |

## Accomplishments

### Task 1: Ureg rework to the 184-byte 9k layout + witness re-key (tracer)

- **dat.rs:** the four u64 segment fields became `u16 ds/es/fs/gs` packed in ONE qword at offset 120 (byte-verbatim with `amd64/include/ureg.h`); `trapno` renamed `type_` at slot 128; `UREG_TRAPNO_OFFSET` re-keyed 19\*8→16\*8 (128), `UREG_CS_OFFSET` 22\*8→19\*8 (152), names kept; the 15-line always-on const-assert block added (ax@0, bp@48, r15@112, quartet 120-126, type@128, error@136, ip@144, cs@152, flags@160, sp@168, ss@176, size 184) mirroring the aarch64 idiom.
- **trap.rs intrcommon:** the carve `subq $((4+15)*8)` → `subq $(16*8)` (15 GP qwords + 1 packed-dseg qword below the vector slot); the four qword segment stores became direct word stores at `15*8+0..6` (exactly 9k syscallentry's store arithmetic, no scratch register); restore + pop re-keyed to `(2+1+15)*8`.
- **Witness re-derivation (CR-01 class):** intrspecial/intrregular's `SWAPPED_CS_OFFSET+24` sites are stub-entry-relative (retaddr@0, error@+8, RIP@+16, CS@+24 — independent of intrcommon's carve), verified unchanged with their comment shape intact; intrcommon's `btr` rides `SWAPPED_CS_OFFSET + UREG_CS_OFFSET` (now 7+152=159, the CS qword's high byte) with a new in-place comment recording the 176→152 re-derivation.
- **Regression (all green, zero aarch64-path edits):** q35 boot-test ×2 consecutive, q35net (virtio IRQ traffic on the new frame — every tick exercises the reworked save/restore), aarch64 virt sibling, check, clippy, port tests (101), x86_64 bin tests (26).

### Task 2: amd64/ABI.md published (XEXEC-01, D-02)

328 lines in the aarch64/ABI.md template's section order, every frozen fact citing its read-verbatim source: title+provenance (Harvey's own 9k k10 tree + toolchain, NOT 9front; the D-01 both-kernels consequence), wire contract (number in BP via RARG=D_BP, args at user SP+8 upward with SP+0 the return-address slot, SYSCALL MSR-routed no-hw-frame, result AX, Sfmask=If only), stub shape (mkfile case-amd64 generator quoted verbatim + the three frozen quirks: nsec NO arg0 spill, seek→_seek, exits→_exits), syscall table (sys.h 0-54 restated, NMOUNT=54 Harvey-only, the loud-unimplemented rule for every other number — the flagged-assumption wording pinned as contract), trap frame (ureg.h quoted whole + the pinned 184-byte offset table with three independent 9k assembly cross-checks), entry state + note delivery (software-built SYSCALL frame, STAR/LSTAR/FMASK MSR contract, touser SYSRETQ register state, handler arg0 through ureg->bp), stack-top policy (UTZERO 2 MiB, INITTEXT 0x200028, INITRND 2 MiB, USTKTOP 0x00007ffffffff000, USTKSIZE 16 MiB — the discretion resolution), and the enforcement appendix (const asserts + dual-arch ureg-check + the D-02b negative control with the u16int quartet packing rule).

### Task 3: ureg-check amd64 arm + negative control + boot-test gate (D-02b)

- **uregcheck.rs:** the amd64 pinned table (26 declarations: 22 u64int fields + the ds/es/fs/gs u16int quartet at slots 15-18 of one qword, packed explicitly, never skipped — Pitfall 9); parser extended for the bare `struct Ureg {` form (no typedef; the bare-struct finder rejects the aarch64 typedef'd form) and `u16int` declarations; `harvey_dir()` error names both headers; `run_check()` checks both arms then runs the negative control; `check_for_boot_test(arch)` requires the arch's own header.
- **Negative control:** mutates an in-memory copy of the REAL amd64 header (cs→cseg) and requires rejection with a named-field diff; a passing mutated copy — or a mutation that cannot apply — is the loud `NEGATIVE CONTROL FAILED` failure.
- **main.rs:** the boot-test precondition runs the ureg-check for BOTH aarch64 and x86_64 (Riscv64 still rejected); the Phase-13 deferral comment retired.
- **Proofs:** 97 xtask tests green (6 new amd64 tests incl. the quartet-removal drift pin and the control leg); live `ureg-check` prints both Match lines + the control-fired line; q35 boot-test green through the gate; `R9_HARVEY_DIR=/nonexistent-coverage` exits 1 with the named amd64 error before qemu spawns; the aarch64 arm is byte-stable (same table, same match line, same drift-diff wording, same tests).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Verify-command re-key] `cargo test -p x86_64 -- --nocapture` cannot run as written**
- **Found during:** Task 1 verify
- **Issue:** the plan's literal verify command fails on ANY host (including at Phase-13 HEAD): the x86_64 crate has `default-target = "x86_64-unknown-none"`, so a host-target bin-test compile hits 596 no_std/host mismatches. The canonical gate is `cargo xtask test`, whose arch-bins leg runs `cargo test -p x86_64 --bins --target <std-supported>` when rustup is present — deliberately skipped in the nix devshell (no rustup).
- **Fix:** ran `cargo xtask test` (port lib: 101 green) PLUS the explicit host-target leg `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` (26 green) — the exact command xtask would issue with rustup present. Semantics preserved: the x86_64 unit tests ran and passed.
- **Files modified:** none (execution-only re-key)
- **Commit:** n/a

**2. [Rule 3 - Environment] check/clippy legs run inside `nix develop`**
- **Found during:** Task 1 verify
- **Issue:** the plan shows `cargo xtask check && cargo xtask clippy` on the bare host; this host has no bare cargo (rustc lives only in the r9 devshell — qemu is on PATH but the toolchain is not).
- **Fix:** ran both inside `nix develop --command sh -c '...'` — same commands, same exit semantics, both green.
- **Files modified:** none
- **Commit:** n/a

**Total deviations:** 2 auto-fixed (2 verify-command environment re-keys). **Impact:** none on scope or guarantees — every plan verify leg ran and passed, with the x86_64 test coverage actually deeper than the literal command could provide.

## Authentication Gates

None.

## Known Stubs

None — no stubs, placeholders, or unwired data paths were introduced.

## Issues Encountered

None beyond the two documented deviations.

## TDD Gate Compliance

N/A — this plan is `type: execute` (no tdd="true" tasks; const-assert and unit-test gates were added as plan content, not TDD cycles).

## Next Plan Readiness

Ready for 14-02 — the frozen frame, the published ABI.md, and the dual-arch enforcement gate are the foundation every ring-3/exec plan builds against.

## Self-Check: PASSED
