---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 21
subsystem: testing
tags: [flake-nix, github-actions, xtask, smp-stress, mem-pressure, aeadloop, palloc-evictor, kernel-stack, aarch64, halted]

# Dependency graph
requires:
  - phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
    provides: "05-27's four -smp 2 wrappers + expect drivers; 05-20's aeadloop=1 loopback server; 05-16/05-17 palloc evictor + page cache; 05-22's -smp 2 kstack-ownership fix"
provides:
  - "flake.nix apps boot-r9-smp-stress, boot-r9-mem-pressure, boot-r9-smp-stress-x64, boot-r9-mem-pressure-x64 wrapping nix/gates/*.sh"
  - "gates.yml jobs running the four stress/pressure gates on both architectures at -smp 2 (explicit TCG, no error masking)"
  - "xtask full-tier registration for the four apps (+ output-dir maps and Phase 1 witness map)"
  - "aead_loop serves a real root directory listing and Tstat (ls '#M' works)"
  - "32 KiB per-proc kernel stack + unmapped overflow-guard page (the #M coroutine server needs the deeper stack)"
affects: [Owner Stop 4, SMP-08, SMP-09, MEM-15, MEM-17, MEM-10]

# Actuals (#2632)
actuals:
  tokens: 24311
  tasks: 1
  commits: 3
  plan_head_before: 0c14fb11461a4ddc6b81dbba842440cd869108f3
  plan_head_after: e5ae8681491a26d55c208ef7b5e16273fd881440

tech-stack:
  added: []
  patterns:
    - "Nix app wiring mirrors mkScriptApp/bootR9RuntimeInputs (aarch64) and bootR9X64RuntimeInputs (x86_64-linux only)"
    - "CI stress/pressure jobs set R9_ACCEL=tcg explicitly (no silent downgrade)"
    - "Synthetic 9P server serves stat(5) records via crate::dat::StatWriter, shared by Tstat and the directory listing"
    - "Kernel stacks must fit the deepest SYNCHRONOUS path: an in-kernel 9P server nests the client's syscall frames, so KSTACK_PAGES follows the loopback's coroutine depth and the lowest page is an unmapped guard"

key-files:
  created: []
  modified:
    - flake.nix
    - .github/workflows/gates.yml
    - xtask/src/gate.rs
    - .planning/milestones/v3.0-phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
    - nix/gates/smoke-r9.sh
    - port/src/aead_loop.rs
    - gates/aarch64/mem-pressure.expect
    - gates/x86_64/mem-pressure.expect
    - aarch64/src/proc.rs
    - aarch64/src/smp.rs
    - .planning/WINDOWS.md

key-decisions:
  - "Tstat + directory listing belong in the synthetic loopback server: `ls '#M'` is a real directory read plus Tstat, not just attach/walk/open/read/clunk"
  - "The smp-stress wild-pc halt is a KERNEL STACK OVERFLOW, not a scheduler/trap defect: the #M loopback is a coroutine server that nests the client's syscall frames, so the 16 KiB donor stack cannot hold the deep path"
  - "KSTACK_PAGES 4 -> 8 plus an unmapped lowest page: an over-deep stack must be a named EL1 fault, never a silent write into a neighbouring device-VA mapping"
  - "mem-pressure's `pinned_skipped` counter cannot be driven from this image's userland (see BLOCKER 2): writable buffers are promoted off the page cache's droppable queue and the only read-only user slice (pwrite's source) is a stack or .bss page, which is never cached"

patterns-established:
  - "9P server stat(5) records built with crate::dat::StatWriter (41-byte fixed part + four length-prefixed strings)"

requirements-completed: [SMP-08, SMP-09]

coverage:
  - id: D1
    description: "flake.nix/CI/xtask expose and dispatch the four -smp 2 stress/pressure apps"
    requirement: SMP-08
    verification:
      - kind: other
        ref: "nix eval .#apps.*.{boot-r9-smp-stress,boot-r9-mem-pressure,...}.program; python3 yaml.safe_load gates.yml (13 jobs); cargo test -p xtask full_"
        status: pass
    human_judgment: false
  - id: D2
    description: "aarch64 smp-stress runs green twice consecutively under HVF and TCG"
    requirement: SMP-09
    verification:
      - kind: e2e
        ref: "nix run .#boot-r9-smp-stress (HVF x2) + R9_ACCEL=tcg nix run .#boot-r9-smp-stress (TCG x2) — two consecutive full passes, all 4 runs exit 0 with SMP_STRESS_OK"
        status: pass
    human_judgment: false
  - id: D3
    description: "aarch64 mem-pressure boots the 8 MiB pool, pins a buffer, re-faults evicted pages, and reports positive evictor counters"
    requirement: MEM-15
    verification:
      - kind: e2e
        ref: "nix run .#boot-r9-mem-pressure (HVF x2): boot/palloc/pin/re-fault pass; `#c/sysstat` reports dropped_text=0 dropped_data=0 pinned_skipped=0"
        status: fail
    human_judgment: true
    rationale: "Three independent gaps: (a) the authored load never exhausts 2048 frames; (b) true exhaustion kills the allocating proc (`fault: phys page alloc failed`) instead of evicting, then the run wedges; (c) `pinned_skipped` needs a syscall to pin a CACHED page, which no userland path in this image can produce. Owner decision required."
  - id: D4
    description: "host check, both clippy legs, xtask test and Miri exit 0 twice consecutively"
    requirement: MEM-17
    verification:
      - kind: other
        ref: "pass1 + pass2: cargo xtask check, clippy --arch aarch64 --config virtshell, clippy --arch x86-64 --config q35cpu, cargo xtask test, cargo xtask miri — all rc=0 in both passes"
        status: pass
    human_judgment: false

duration: ~3 h 40 min (two full local passes + kernel root-cause debug session)
completed: 2026-11-27
status: halted
---

# Phase 05 Plan 21: Two-CPU Stress & Pressure Gate Wiring Summary

**Wired Plan 05-27's four `-smp 2` stress/pressure gates into `flake.nix`, CI and `xtask`, root-caused the deterministic smp-stress halt to a kernel-stack overflow and fixed it (32 KiB stack + overflow guard) — the stress gate is now green twice consecutively on HVF and TCG — but HALTED before asking for Owner Stop 4 because the mem-pressure gate still reports zero evictor activity and its `pinned_skipped` arm is not reachable from userland as designed.**

## Performance

- **Duration:** ~3 h 40 min (gate runs, a kernel debug session, two consecutive full local passes)
- **Completed:** 2026-11-27 (halted)
- **Tasks:** 1 of 2 (Task 1 implementation committed; stress leg GREEN, pressure leg RED)
- **Files modified:** 11

## Accomplishments

- `flake.nix` exports `boot-r9-smp-stress`, `boot-r9-mem-pressure` (all systems), and `boot-r9-smp-stress-x64`, `boot-r9-mem-pressure-x64` (x86_64-linux), each wrapping Plan 05-27's `nix/gates/*.sh` with the established `mkScriptApp` supply contract.
- `.github/workflows/gates.yml` gains the four stress/pressure jobs on both architectures at `-smp 2`, explicit `R9_ACCEL=tcg`, and no error masking (T-05-37).
- `xtask/src/gate.rs` registers the four apps in the `cargo xtask gate --full` tier (with output-dir maps); the Phase 1 witness map is mirrored so `cargo test -p xtask` stays green.
- `port/src/aead_loop.rs` now serves a real root directory listing and `Tstat` (built with the shared `StatWriter` shape); `ls '#M'` enumerates `data0..data7` and `stat` succeeds. Added `aead_loop_root_enumerates_and_stats`.
- `nix/gates/smoke-r9.sh` asserts `aeadloop: active` is ABSENT on a default boot (SMP-13) — verified green on `boot-r9-smoke`.
- **Kernel fix:** the deterministic smp-stress halt (`InstructionAbortSameEl pc=far=0x9b7728f0`) was a **kernel-stack overflow**. The in-kernel `9P2000.aead` loopback is a *coroutine* server: the client's syscall frames stay live while the server's dispatch runs on the same stack, so `KSTACK_PAGES=4` (16 KiB) could not hold the deepest path. Raised to 8 pages (32 KiB) and left the lowest stack page **unmapped as a guard**, so an over-deep stack now takes a named EL1 fault instead of silently corrupting a sibling proc's stack.
- `boot-r9-smp-stress` is now **green twice consecutively on HVF and on TCG** (SMP_STRESS_OK, zero deadlocks/watchdog/rank panics); `boot-r9-smoke`, `boot-r9-tier1` and `boot-r9-cpu` remain green with the new stack budget.

## Task Commits

1. **Task 1a: Wire gates into flake/CI/xtask + gate-server/driver fixes** — `0da8b07` (feat)
2. **Task 1b: Halted-state record (first pass)** — `80d1711` (docs)
3. **Task 1c: Kernel-stack overflow fix (32 KiB + guard page)** — `e5ae868` (fix)

**Plan metadata:** committed with this SUMMARY (docs).

## Files Created/Modified

- `flake.nix` — four new Nix apps.
- `.github/workflows/gates.yml` — four new CI jobs (both arches, explicit TCG).
- `xtask/src/gate.rs` — full-tier lists/maps/tests for the four apps.
- `.planning/milestones/v3.0-phases/01-stage-p0-cross-build-harness/witness/run-witness.sh` — mirrored the two new x86_64 rows.
- `nix/gates/smoke-r9.sh` — SMP-13 default-dormant negative assertion.
- `port/src/aead_loop.rs` — directory listing + Tstat + unit test.
- `gates/aarch64/mem-pressure.expect` — stage order + `hsleep 60`.
- `gates/x86_64/mem-pressure.expect` — stage order.
- `aarch64/src/proc.rs` — `KSTACK_PAGES` 4 → 8, unmapped guard page, doc rationale.
- `aarch64/src/smp.rs` — `moribund_reap` array sized from `proc::KSTACK_PAGES`.
- `.planning/WINDOWS.md` — #17/#20 closed with evidence, #21 opened (x86_64 live legs).

## Decisions Made

- The synthetic loopback server must answer `Tstat` and a directory `Tread`; `ls '#M'` is not just attach/walk/open/read/clunk.
- Register the gates in `gate.rs` (where the full-tier list actually lives) rather than `main.rs` (the plan's named file); this is the real `cargo xtask gate` seam.
- The kstack budget follows the deepest *synchronous* path, not the donor's `INTERRUPTSTACKSZ`: Plan 9 never nests a whole 9P server inside a client syscall, this kernel does.
- Keep the task commits even though the pressure gate is RED: the wiring, the server/driver fixes and the stack fix are correct, unit-tested, and required for the next attempt.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `aead_loop` refused `ls '#M'` (missing Tstat + directory listing)**
- **Found during:** Task 1 first `nix run .#boot-r9-smp-stress`
- **Issue:** `ls '#M'` printed `devmnt: server error: aeadloop: request refused`; `serve()` handled attach/walk/open/read/clunk only, and the root `Tread` returned an empty vector. `ls` needs `Tstat` and a stat-record directory listing.
- **Fix:** Added `stat_record()` (via `crate::dat::StatWriter`) and `directory_bytes()`, a `Msg::Tstat` arm, and a directory-aware `Tread`; added a unit test.
- **Files modified:** `port/src/aead_loop.rs`
- **Verification:** `cargo test -p port aead_loop` 5/5 incl. the new test; the gate now reaches the stress stage.
- **Committed in:** `0da8b07`

**2. [Rule 1 - Bug] mem-pressure driver stage order was impossible**
- **Found during:** Task 1 `nix run .#boot-r9-mem-pressure`
- **Issue:** The driver waited for `palloc: managed_frames=...` AFTER `smp: cpu 1 online`, but palloc prints at memory init (before CPU 1 online), so the stage could never match and the gate timed out at `palloc`.
- **Fix:** Reordered to banner → palloc → smp-online → paqfs (both arches).
- **Files modified:** `gates/aarch64/mem-pressure.expect`, `gates/x86_64/mem-pressure.expect`
- **Verification:** the gate now boots through to the pressure worker and re-fault stages.
- **Committed in:** `0da8b07`

**3. [Rule 1 - Bug] aarch64 mem-pressure used a non-existent `sleep`**
- **Found during:** Task 1 (gate wedged at `pinned-bytes`)
- **Issue:** The arm64 image stages `sleep(1)` as `/arm64/bin/hsleep`; the driver's bare `sleep 60` never resolved, so the pipe closed and `pinnedok` never arrived.
- **Fix:** `hsleep 60` in the aarch64 driver (the x86_64 chain image has no sleep at all — recorded as an open item).
- **Files modified:** `gates/aarch64/mem-pressure.expect`
- **Verification:** the pinned reader now delivers `pinnedok`.
- **Committed in:** `0da8b07`

**4. [Rule 3 - Blocking] xtask registration lives in `gate.rs`, not `main.rs`**
- **Found during:** Task 1
- **Issue:** The full-tier app list is `FULL_APPS_{X86_64,AARCH64}` in `xtask/src/gate.rs`; `main.rs` only dispatches.
- **Fix:** Registered the four apps in `gate.rs`, updated the three gate tests (lengths/lists), and mirrored the two x86_64 rows in the Phase 1 witness map (the test pins byte-equality).
- **Files modified:** `xtask/src/gate.rs`, `.planning/milestones/v3.0-phases/01-stage-p0-cross-build-harness/witness/run-witness.sh`
- **Verification:** `cargo test -p xtask full_` 3/3; `cargo xtask check` clean.
- **Committed in:** `0da8b07`

**5. [Rule 2 - Missing Critical] SMP-13 default-dormant assertion**
- **Found during:** Task 1
- **Issue:** No leg asserted the loopback server stays dormant without `aeadloop=1`.
- **Fix:** `nix/gates/smoke-r9.sh` greps the capture for the active marker and fails on a default boot (guarded for an explicit `R9_BOOTARGS` opt-in).
- **Files modified:** `nix/gates/smoke-r9.sh`
- **Verification:** `nix run .#boot-r9-smoke` green (the negative arm runs and passes).
- **Committed in:** `0da8b07`

**6. [Rule 1 - Bug] kernel-stack overflow behind the deterministic smp-stress halt**
- **Found during:** Task 1 kernel debug session (after `0da8b07`)
- **Issue:** `boot-r9-smp-stress` halted deterministically at `InstructionAbortSameEl pc=far=0x9b7728f0` once the `#M` read workers ran. Bisected to the *baseline* loopback code (the halt reproduces with the pre-`Tstat` `aead_loop.rs`), then to a **kernel stack overflow**: instrumenting the UB check showed `UserSliceMut.len` holding sealed-packet bytes (i.e. the kstack had been overwritten). The in-kernel 9P server nests the client's syscall frames (`sys_pread`/`exit_tail` → `devmnt` → `SealedWriteIo` → `LoopMntIo` → `LoopServer::handle_one` → `recv_sealed` → `serve` → `Session::receive/seal`), which does not fit 16 KiB; the overflow ran into the neighbouring device-VA mapping (a sibling proc's stack).
- **Fix:** `KSTACK_PAGES` 4 → 8 (32 KiB) with the rationale documented, the LOWEST kstack page left unmapped as an overflow guard, and `PerCpu::moribund_reap`'s PA array sized from `proc::KSTACK_PAGES` (the hard-coded `4` would have silently truncated the reap).
- **Files modified:** `aarch64/src/proc.rs`, `aarch64/src/smp.rs`
- **Verification:** `boot-r9-smp-stress` green HVF ×2 and TCG ×2 (two consecutive full passes); `boot-r9-smoke`, `boot-r9-tier1`, `boot-r9-cpu` green; unit tests and Miri green in both passes.
- **Committed in:** `e5ae868`

---

**Total deviations:** 6 auto-fixed (5 Rule 1, 1 Rule 2, plus the Rule 3 registration seam)
**Impact on plan:** All six are required for the gates to run at all. Deviation 6 clears the smp-stress blocker; the mem-pressure blockers below remain open.

## Issues Encountered

### BLOCKER — mem-pressure reports zero evictor activity (MEM-15 unproven)

The gate now boots the 8 MiB pool, pins the pipe reader, re-faults `/lib/namespace` byte-identically, and reads `#c/sysstat`, which reports:

```
evictor: dropped_text=0 dropped_data=0 pinned_skipped=0
```

Three independent problems, all established by direct measurement:

1. **The authored load never exhausts the pool.** `alloc_clean` only runs the evictor once `clean` and `dirty` are empty. `R9_PALLOC_MAX_FRAMES` was swept over 2048 (default), 1536 and 1200 — all report 0/0/0, i.e. the live set stays below the cap (the boot chain is small; repeated exec of the same three binaries reuses shared text, so the churn adds almost nothing).
2. **True exhaustion kills the allocator instead of evicting, then wedges.** Driving a real allocation burst (`hseq 1 400000 | hsort`) produced `fault: phys page alloc failed` → `user fault: unresolvable … pid=6 name=hseq` and then the pipe-RPC wedge (`paqfs` blocked, no input waiters). At `R9_PALLOC_MAX_FRAMES=512` the boot itself dies with `DataAbortSameEl far=0x20118`; at 1024 it boots and wedges after the reader stage. So the live evictor reclaims nothing when it matters — MEM-15/MEM-16 are **not** proven end-to-end.
3. **`pinned_skipped` is not reachable from this image's userland.** The counter only rises when `PageCache::select_eviction_victim` meets a *pinned* candidate from the droppable queue. Writable buffers (`UserSliceMut`: pread targets, the blocked pipe reader) are promoted to `AllRw`+dirty at construction and leave the droppable queue; the only read-only user slice is `pwrite`'s source, and every writer in this image writes from a stack or `.bss` page — neither is ever registered in `palloc.hash`. The 05-27 driver's premise ("a blocked pinned reader yields `pinned_skipped`") is therefore structurally unachievable.

**Owner decision required** (Rule 4 — semantic/design scope, not an auto-fix):
- (a) fix the exhaustion path so the evictor actually reclaims (the wedge is the actionable defect), and
- (b) either give the gate a real pinned-droppable shape (e.g. a staged probe binary that `pwrite`s from its own read-only text) or revise the `pinned_skipped > 0` criterion in Plan 05-27/05-21.

### Deployment gaps

- The x86_64 gates cannot run on this aarch64 macOS host (Linux-only `grub-mkrescue`/authdomain venue); they remain for CI TCG and the owner's Linux KVM host (D-33). Recorded as WINDOWS #21.
- The x86_64 mem-pressure driver still uses `sleep 60`, which the minimal chain image does not stage (WINDOWS #19).

### Verification actually run (two consecutive full passes, IG-V01)

| Leg | Pass 1 | Pass 2 |
|-----|--------|--------|
| `nix run .#boot-r9-smp-stress` (HVF) | rc=0 (9 s) | rc=0 (10 s) |
| `R9_ACCEL=tcg nix run .#boot-r9-smp-stress` | rc=0 (10 s) | rc=0 (10 s) |
| `nix run .#boot-r9-mem-pressure` (HVF) | rc=1 (99 s) | rc=1 (98 s) |
| `cargo xtask check` | rc=0 | rc=0 |
| `cargo xtask clippy --arch aarch64 --config virtshell` | rc=0 | rc=0 |
| `cargo xtask clippy --arch x86-64 --config q35cpu` | rc=0 | rc=0 |
| `cargo xtask test` | rc=0 (44 s) | rc=0 (10 s) |
| `cargo xtask miri` | rc=0 (394 s) | rc=0 (480 s) |
| `nix run .#boot-r9-smoke` (full aarch64 quick pass) | rc=0 | — |
| `nix run .#boot-r9-tier1` | rc=0 | — |
| `nix run .#boot-r9-cpu` | rc=0 | — |

Captures: `/tmp/05-21-suite-pass{1,2}/<leg>.log`; gate captures under `target/gates/{smp-stress-r9,mem-pressure-r9,smoke-r9,tier1-r9,cpu-r9}/`.

## Known Stubs

- `gates/x86_64/mem-pressure.expect` — pinned reader invokes `sleep`, absent from the amd64 chain image (`xtask/manifests/amd64/chain.toml`). Registered as WINDOWS #19.
- `gates/aarch64/mem-pressure.expect` — the evictor-counter stage cannot pass until BLOCKER items (a)/(b) above are resolved. Registered as WINDOWS #18.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**NOT ready to approve Owner Stop 4.** The smp-stress blocker is closed (kernel-stack fix, twice-consecutive green on HVF and TCG) and the wiring/CI/xtask/host-suite legs are green twice. Phase 5's MEM-15/MEM-16 proof still needs the owner's call on the mem-pressure exhaustion defect and on the `pinned_skipped` criterion before the pressure gate can go green. The x86_64 stress/pressure live legs remain for CI TCG + the owner's KVM host (D-33).

## Self-Check: PASSED

- All 11 modified files exist on disk.
- Commits `0da8b07`, `80d1711`, `e5ae868` are ancestors of HEAD.
- Plan status is `halted` by design: the pressure leg is RED on the blockers above, so Owner Stop 4 was not self-approved.

## Post-script: mem-pressure resolution (owner-approved debug session, 2026-11-27)

The owner approved fixing BLOCKER items (a)/(b) in place.  Debug session
`.planning/debug/resolved/aarch64-palloc-evictor.md` (slug
`aarch64-palloc-evictor`) carries the full evidence; the outcome:

- **Root causes (all three had to be fixed).** (1) `alloc_clean` reclaimed
  ONE frame per evictor attempt; the frame returns to the shared dirty pool
  and the sibling CPU took it before the caller's retry, so a starving
  caller burned `MAX_EVICT_ATTEMPTS` and returned `OutOfSpace` while the
  cache was still reclaimable (measured: `freed … pool_after=(dirty=1
  active=2047)`, next attempt `dirty=0 active=2048`).  (2) The droppable
  FIFO had no mappedness preference and, once cold pages were gone, evicted
  hot text pages of running processes — 377,377 refaults of `va=0x18000`
  (evict→refault livelock).  (3) The authored load never exhausted the pool
  (exec churn) or demanded ~100x it (`hseq 1 400000`), and
  `pinned_skipped > 0` is structurally unreachable from this image's
  userland (05-21 BLOCKER 3).
- **Fix.** `EVICT_BATCH_FRAMES=16` so a reclaiming caller keeps a margin of
  its own frames; `select_eviction_victim` prefers unmapped (cold, rmap
  empty) pages and falls back to the oldest mapped one; `#c/sysstat` gained
  `palloc.hash:`/`palloc.pool:` diagnostics plus `evict_calls`, `reclaimed`
  and `free_failures`; the aarch64 driver now drives one bounded
  `hseq 1 44000 | hsort > /dev/null` burst (`R9_PRESSURE_LINES`) and asserts
  `dropped_text > 0`, `dropped_data > 0`, `evict_calls > 0`,
  `reclaimed > 0`, `free_failures == 0`, forbidding `phys page alloc
  failed` in the capture.
- **Criterion change.** `pinned_skipped > 0` is removed from both
  mem-pressure drivers (reported, not asserted).  Pinning is proven by the
  host integration test
  `port/src/pagecache.rs::pinned_cached_page_is_skipped_until_unpinned`,
  which pins a real cached frame through `PageAlloc::pin_frame` (the exact
  `UserSlice` PinGuard call) and requires the evictor to skip and count it,
  then take it once unpinned.  The background pinned-reader stage was
  removed: it could not pin a cached page, changed the pressure baseline by
  starting extra shells, and added exec-after-pressure surfaces.
- **Evidence (real numbers).** 4 consecutive passes, 3x HVF + 1x TCG,
  identical counters each run:
  `palloc: managed_frames=2048` /
  `evictor: dropped_text=20 dropped_data=28 pinned_skipped=0 evict_calls=3
  reclaimed=48 free_failures=0`, re-fault line intact, `MEM_PRESSURE_OK`,
  no `phys page alloc failed` / `unresolvable` marker.  Host suite:
  `cargo xtask check` rc=0, `cargo xtask test` rc=0 (port 208 tests).
- **Still open.** The x86_64 mem-pressure live leg remains unvalidated on
  this aarch64 host (Linux-only venue, WINDOWS #21); its driver drops the
  same unreachable `pinned_skipped > 0` assertion but keeps its original
  exec-churn workload, so it is not yet known green.
