---
phase: 05-shell-boot-one-command-integration
plan: 01
subsystem: kernel
tags: [r9, aarch64, trap-return, note-delivery, demand-paging, stack-growth, boot-test-harness, syscall-abi, qemu]

# Dependency graph
requires:
  - phase: 04-r9-userspace-exec-harvey-r9-abi
    provides: staged-init supply chain (stage-build.sh → out/ → xtask initrd), the exec/virtmulti/virtnotes profiles, the note machinery (deliver_notes, NOTIFY/NOTED), and the deferred-#2 stale-log poison report
  - phase: 03-r9-qemu-virt-platform-bring-up
    provides: the zero-dep boot-test harness contract (D-11) and the virt platform
provides:
  - a root-caused, statistically buried note-delivery fix on r9 main (83cbbbe) — the shell path may lean on notes
  - a hygiene-hardened boot-test harness: fresh serial log per run + process-group kill (c2f141e)
  - the pread/pwrite `long n` 32-bit wire fix (61e6a70) — donor-faithful ABI
  - the stacksmoke staged gate: demand-paged stack growth is execution-evidenced on every virtexec run (66e5d1a + harvey a3bf8f71e)
affects: [05-02 (real boot chain depends on notes + stack growth), all later shell-phase plans, INT-02 smoke reliability]

# Actuals (#2632)
actuals:
  tokens: 12474      # chars/4 over authored files changed (r9 32873 + harvey code 5721 + SUMMARY ~10000 ≈ 49.9k chars); excludes the 194647 bytes of captured serial evidence logs (artifacts, not authored work)
  tasks: 3
  commits: 6         # measured at SUMMARY write: r9 5 (54e8c10..d70e6c7) + harvey 1 (ec2f29543..a3bf8f71e); the harvey metadata commit rides on top
plan_head_before: {r9: "54e8c1079933a2f6b3e1d6b1e04e3c01c69c9a5e58", harvey: "ec2f29543a8c2fe982605a5459a2a05e3a8c558b"}
plan_head_after: {r9: "d70e6c7d16d6042b865fac33b34af5132072437c", harvey: "a3bf8f71e"}

# Tech tracking
tech-stack:
  added: []   # no new dependencies anywhere (D-11 contract held: kill(1) is POSIX base, not a package)
  patterns:
    - "trap_return tail: no GPR may be touched after the register pops — the final sp is frame+288 for both origins (every restored frame sits at entry-stack-top − 288 by construction)"
    - "Plan 9 C dialect: long is 32-bit (7c SZ_LONG=4) on every arch; the kernel masks int/long-width syscall args to the slot's low 32 bits (donor va_arg semantics)"
    - "harness hygiene: destroy the serial log pre-spawn; spawn qemu as process-group leader; kill the group at teardown"

key-files:
  created:
    - build/aarch64/stacksmoke.c
    - .planning/phases/05-shell-boot-one-command-integration/05-01-evidence/ (3 pre-fix failure logs)
  modified:
    - ../r9/xtask/src/main.rs (D-16 hygiene + D-15 marker)
    - ../r9/aarch64/src/trap.S (the note-race fix)
    - ../r9/aarch64/src/proc.rs (comment corrections — CURKSTACK is now a documentary record)
    - ../r9/aarch64/src/syscall.rs (pread/pwrite n masking + width comment)
    - ../r9/HowItWorks.md (material-change section + two stale bullets)
    - build/aarch64/init.c (child D)
    - build/aarch64/stage-build.sh (STAGE_SRCS)
    - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md (item #2 resolved)

key-decisions:
  - "trap_return tail fix: uniform frame+288 final-sp step instead of the CURKSTACK load — semantically identical by the frame-placement invariant, and removes the post-pops x9 scratch use that was the flake's root cause"
  - "the silent-print bug was r9-side (full-64-bit slot read of a 32-bit wire arg), NOT Harvey libc — _fmtFdFlush's int-n store is donor-conforming; the heritage file stays pristine"
  - "CURKSTACK stays in proc.rs as the entry-stack invariant's documentary record; nothing reads it"

patterns-established:
  - "D-14 debug-first pattern: capture the failing log on the pre-fix base BEFORE fixing, name one mechanism in the fix commit, then bury it statistically (20+20 consecutive clean)"
  - "syscall width audit rule: any handler reading an int/long-width arg must mask to 32 bits; pointer/vlong args keep the full slot"

requirements-completed: [INT-01]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "D-16 harness hygiene: fresh serial log per run, stale-socket cleanup, process-group kill on teardown"
    requirement: INT-01
    verification:
      - kind: automated_ui
        ref: "cargo xtask boot-test --config virt with a pre-seeded sentinel log: sentinel destroyed, fresh banner present"
        status: pass
      - kind: automated_ui
        ref: "pgrep -f 'qemu-system-aarch64.*virt' empty after a completed run (no orphan qemu)"
        status: pass
      - kind: unit
        ref: "cargo test -p xtask (19/19)"
        status: pass
    human_judgment: false
  - id: D2
    description: "D-14 note-timeout race root-caused (trap_return tail's post-restore x9 clobber) and fixed; statistically buried"
    requirement: INT-01
    verification:
      - kind: integration
        ref: "20/20 virtnotes + 20/20 virtmulti consecutive clean, zero note-timeout lines, on the final tree (twice: pre-width-fix and post)"
        status: pass
      - kind: integration
        ref: "full profile fan: virt + virtexec + virtmulti + virtnotes + virtpanic all PASS; ureg-check 35/35"
        status: pass
      - kind: other
        ref: "pre-fix failure logs captured: 05-01-evidence/prefix-note-timeout-virtnotes-run7.log + two wild-write logs"
        status: pass
    human_judgment: false
  - id: D3
    description: "D-15 staged half: stacksmoke binary forces multiple demand-paged stack page-ins and prints its depth-64 marker under virtexec"
    requirement: INT-01
    verification:
      - kind: integration
        ref: "grep 'stacksmoke: depth 64 ok' + 'fault: stack growth' ×3 in target/boot-test-virtexec.log; 5/5 runs"
        status: pass
      - kind: integration
        ref: "R_MAGIC 00008c47 on build/aarch64/out/stacksmoke; all pre-existing virtexec markers intact; virtmulti/virtnotes green"
        status: pass
    human_judgment: false
  - id: D4
    description: "pread/pwrite's long n masked to 32 bits (donor va_arg fidelity) — the stacksmoke-exposed ABI deviation"
    requirement: INT-01
    verification:
      - kind: integration
        ref: "same full-fan green run set as D2/D3 (all profiles pass with the masking in the tree)"
        status: pass
    human_judgment: false

# Metrics
duration: 64 min
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 01: Wave-0 — note-race root-cause fix, harness hygiene, stacksmoke gate Summary

**The note-timeout flake is root-caused (trap_return's tail clobbered the restored x9 on every EL0t-bound return) and buried under 40/40 clean runs; the boot-test harness now starts every run against a fresh log and kills qemu by process group; stacksmoke's depth-64 marker rides the real pwrite path under demand paging — and its bring-up exposed and fixed a donor-ABI deviation (pread/pwrite's 32-bit `long n` read full-width).**

## Performance

- **Duration:** 64 min
- **Started:** 2026-11-02T23:45:06Z
- **Completed:** 2026-11-03T00:49:06Z
- **Tasks:** 3
- **Files modified:** 11 (5 r9, 6 harvey)

## Accomplishments

- **D-14 satisfied — the note race is root-caused and statistically buried.** Pre-fix failures captured on the current base BEFORE the fix (debug discipline): `05-01-evidence/prefix-note-timeout-virtnotes-run7.log` (the named symptom) plus two wild-write variants (`prefix-wildwrite-virtnotes-run3.log`, `prefix-wildwrite-virtmulti-run5.log`). Root cause, one named mechanism: `trap.S`'s shared `trap_return` tail restored x9 from the frame (`ldp x8, x9, [sp, #64]`) and then re-used x9 as scratch — the psr re-read and the EL0t `CURKSTACK` load — so every EL0t-bound `eret` resumed userland with x9 = the current proc's kernel-stack top. A tick landing with x9 live destroyed it: a pointer in x9 stored through the kernel-stack VA (smokefork killed at `addr=0xffff90000000f008` = its own kstack top + 8), and smokenote's loop counter in x9 became huge, exiting the spin at its ceiling (`note-timeout`). Fix: the tail's final sp is now `frame + 288` for BOTH origins (every restored frame sits at entry-stack-top − 288 by construction — EL0 entries via the SP_EL1 invariant, EL1h entries by the entry's own `sub`, fork/exec seeds by placement), so no register is touched after the pops and the CURKSTACK load (and its clobber) is gone. Frozen TrapFrame layout untouched (ureg-check 35/35); WR-04/WR-05 guards stay. **Verification: 20/20 virtnotes + 20/20 virtmulti consecutive clean, zero `note-timeout` lines — twice (pre- and post-width-fix tree); at the pre-fix ~1/3 rate, 20 clean pairs hide the bug with probability < 0.03%.**
- **D-16 closed r9-side — harness hygiene.** The boot-test harness destroys the profile's serial log BEFORE spawning qemu (the stale-log poison that bit the 04-05 verify can never satisfy a marker wait again), spawns qemu as its own process-group leader (std `process_group(0)`, zero new deps per the Phase-3 D-11 contract), and kills the whole group at teardown via `kill(1) -KILL -- -<pgid>` with `child.kill()` fallback — one teardown site covers pass/fail/timeout. Verified: sentinel-seeded log destroyed on rerun; no qemu survives a completed run; xtask tests 19/19.
- **D-15 staged half — stack growth is execution-evidenced.** `build/aarch64/stacksmoke.c` (Plan 9 C: tabs, nil, exits, block comments) recurses depth 64 with a 256-byte live frame per level (~16 KiB beyond the two eager seed pages), accumulates a checksum into a static, prints `stacksmoke: depth 64 ok, checksum %lud` through the real pwrite path, and `exits(nil)`. The staged init rforks it as child D (existing children/markers byte-identical; init prints nothing new). Every virtexec run now shows 3 `fault: stack growth` page-ins (0x7fffffffd000/c000/b000) and the marker; 5/5 determinism sweep green; sibling profiles (virtmulti/virtnotes) stay green.
- **Stacksmoke's real value landed immediately**: its deep frames polluted reused stack slots with saved pointers, exposing that r9's sys_pread/sys_pwrite read the `long n` slot full-width while 7c (correctly, per the dialect) stores it 32-bit — `long` is 32-bit in Plan 9 C on every arch (7c `SZ_LONG=4`), and the donor kernel's `va_arg(list, long)` reads only the slot's low half. Fixed donor-faithfully by masking n to 32 bits in both handlers.

## Task Commits

Each task was committed atomically (two-repo convention — r9 commits land on `main` of `requaos/r9`):

1. **Task 1: D-16 harness hygiene** — r9 `c2f141e` (feat)
2. **Task 2: D-14 note-race root-cause fix** — r9 `83cbbbe` (fix; message names the mechanism + the 40/40 count)
3. **Task 3 width fix (found by stacksmoke bring-up)** — r9 `61e6a70` (fix)
4. **Task 3: stacksmoke staged gate, r9 side** — r9 `66e5d1a` (feat)
5. **Task 3: stacksmoke binary + init child D + stage-build + evidence logs, harvey side** — harvey `a3bf8f71e` (feat)
6. **r9 doc update (header-mandated)** — r9 `d70e6c7` (docs)

**Plan metadata:** (the harvey docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS + phase-04 deferred-item resolution)

## Files Created/Modified

- `build/aarch64/stacksmoke.c` — the D-15 deep-recursion smoke binary
- `build/aarch64/init.c` — rforks child D (`/stacksmoke`), awaits 4 records; purely additive
- `build/aarch64/stage-build.sh` — STAGE_SRCS gains stacksmoke (host + guest lists)
- `.planning/phases/05-shell-boot-one-command-integration/05-01-evidence/` — the three pre-fix failure logs (committed debug evidence)
- `../r9/xtask/src/main.rs` — pre-spawn log/socket destruction, process-group spawn + `kill_process_group` teardown, INITRD_FILES + Exec-profile marker
- `../r9/aarch64/src/trap.S` — the note-race fix (uniform frame+288 final-sp; no post-pops scratch)
- `../r9/aarch64/src/proc.rs` — comment corrections (CURKSTACK now documentary)
- `../r9/aarch64/src/syscall.rs` — pread/pwrite n masked to 32 bits + width-discipline comment records long=32-bit
- `../r9/HowItWorks.md` — new 05-01 section + two stale bullets corrected
- `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md` — item #2 marked resolved

## Decisions Made

- **Fix mechanism choice (trap.S):** the uniform `frame + 288` final-sp step over reworking the CURKSTACK load with deferred-register dances — semantically identical under the frame-placement invariant, strictly simpler, and removes the clobber class entirely. CURKSTACK stays in proc.rs as the invariant's documentary record (nothing reads it).
- **The width bug was r9-side, not libc-side.** An intermediate hypothesis put the fix in Harvey's `_fmtFdFlush` (long n) — reverted when the donor evidence (7c `SZ_LONG=4`; `va_arg(list, long)` reads the slot's low half) showed the heritage store is conforming and r9's full-width read was the deviation. Harvey's libc stays byte-pristine against the donor.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] r9 sys_pread/sys_pwrite read the 32-bit `long n` wire arg full-width**
- **Found during:** Task 3 (stacksmoke bring-up — the marker print vanished deterministically after the deep recursion)
- **Issue:** 7c stores a 32-bit-typed arg (int/long) with a 32-bit store, leaving the slot's upper half as stale stack content; stacksmoke's deep frames polluted reused slots with saved stack pointers, so r9 read n as e.g. `0x00007FFF0000001C` (~128 TiB), failed validation, and print() silently dropped the write. Shallow-stack smokes never saw it (fresh demand-paged pages are zeroed).
- **Fix:** mask n to 32 bits in both handlers (donor's `va_arg(list, long)` low-half read; verified against 9front@350266104f `arm64/include/u.h` and vendored 7c `SZ_LONG=4`), with the width-discipline comment corrected.
- **Files modified:** `../r9/aarch64/src/syscall.rs`
- **Verification:** marker deterministic (5/5 virtexec), full profile fan green, ureg-check 35/35, 40/40 note gate re-run clean on the final tree.
- **Committed in:** `61e6a70`

**2. [Rule 3 - Blocking] stacksmoke's marker never printed (blocked Task 3's acceptance)**
- Same root issue as #1 (the write of the marker was the casualty); the n-masking fix unblocked Task 3. No separate change.

---

**Total deviations:** 1 auto-fixed (Rule 1; the Rule-3 blockage was the same bug)
**Impact on plan:** The deviation is the D-15 smoke doing its job — it exposed a real ABI-fidelity bug that the shell phase would otherwise have hit through rc's deep parser frames. No scope creep; no plan content changed.

## Issues Encountered

- **My own diagnostic instrumentation broke the build silently once** (a `frame.pc` private-field access under a `>/dev/null` redirect hid the compile error and left a stale kernel/log). Recovered by keeping builds visible and gating on binary mtimes; no tree state affected.
- The lima guest's rsync-back covers only `build/aarch64/out/`, and my probe-driven `vm-run.sh` invocations re-rsync the host checkout (clobbering the guest's rebuilt libc.a with the host's stale one between runs) — noted for future probe work; stage-build.sh itself is unaffected (it rebuilds libc in-guest every run).

## User Setup Required

None - no external service configuration required.

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, or trust-boundary surface beyond the plan's threat model. (T-05-01 mitigations held: ureg-check 35/35, WR-04/WR-05 intact, full fan green. T-05-02: the kill targets only the harness-spawned process group; the orphan-verify leg proves no over/under-kill. T-05-SC: zero package installs.)

## Next Phase Readiness

- The kernel stream starts on a trustworthy harness: notes are shell-safe (D-14 closed), the harness can't certify stale evidence (D-16), and stack growth has execution evidence (D-15 staged half).
- The width audit rule is recorded for the Phase-5 syscall growth (mask int/long-width args; pointer/vlong full-width) — the donor trace for the real boot chain should re-verify widths as new handlers land.
- r9 HEAD: `d70e6c7` (main). Harvey HEAD: `a3bf8f71e` + the metadata commit.
- Ready for the next Wave-0/1 plans (donor trace of the real chain; namespace machinery).

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: stacksmoke.c, init.c, stage-build.sh, 3 evidence logs, r9 trap.S/proc.rs/syscall.rs/xtask/HowItWorks.md — all FOUND on disk and committed.
- Commits: r9 c2f141e, 83cbbbe, 61e6a70, 66e5d1a, d70e6c7; harvey a3bf8f71e — all FOUND in git log.
- Plan-level <verification>: all three task gates re-verified at summary time (sentinel/orphan/tests; 40/40 + fan + ureg-check; magic + marker + siblings).
