---
phase: 261103-giq
plan: 01
subsystem: infra
tags: [rust, std-only, 9p2000, rsp, mkfs, serial-driver, lima, qemu, flake, aarch64, harness]

requires:
  - phase: 02-aarch64-toolchain-libc-9front-donor
    provides: the five Python harness tools (srv9/drive9/gdbpoke9/mkfs9), the donor boot/run-gate/subset gates, lima guest + pinned donor tree
  - phase: 05-shell-boot-one-command-integration
    provides: trace-syscalls.py --boot-chain rig, the committed 04-01/05-03 trace artifacts

provides:
  - build/harness — ONE std-only Rust binary `harness` with subcommands srv9, drive9, gdbpoke9, mkfs9, trace-syscalls (drop-in CLI-compatible with the deleted Python tools)
  - flake.nix packages.harness on all evaluable systems + devshell cargo/rustc + HARNESS_BIN export (nix-store-first, D-04/D-06)
  - all four build/aarch64 shell callers rewired to HARNESS_BIN; gatedrive9/subsetdrive9 heredocs replaced by drive9 --steps TSV tables
  - guest disk/mkfs as a thin wrapper exec'ing the compiled aarch64-linux harness mkfs9 (D-05)
  - five D-07 equivalence gates green with the .py files deleted

affects: [any future build/aarch64 harness work, r9-integration gates that reuse srv9/drive9]

actuals:
  tokens: 101471
  tasks: 3
  commits: 14

tech-stack:
  added: [rust (stable nixpkgs cargo/rustc in the devshell), build/harness cargo crate]
  patterns:
    - "std-only Rust for host tooling (empty [dependencies] + committed zero-dep Cargo.lock → sandboxed network-free nix build; supply-chain mitigation T-GIQ-SC)"
    - "harness subcommand dispatch with 0/1/2 exit-code discipline; byte-exact scraped markers ('srv9: serving', 'poked', 'processing'/'file system made', '== PASS: …')"
    - "drive9 --steps FILE: tab-separated post-shell step tables replace per-gate generated driver heredocs"
    - "srv9 per-connection fid tables + shared vmap; Twstat parses the doubly size-prefixed Dir wire shape"

key-files:
  created:
    - build/harness/Cargo.toml
    - build/harness/Cargo.lock
    - build/harness/src/main.rs
    - build/harness/src/regex.rs
    - build/harness/src/srv9.rs
    - build/harness/src/gdbpoke9.rs
    - build/harness/src/mkfs9.rs
    - build/harness/src/console.rs
    - build/harness/src/drive9.rs
    - build/harness/src/trace.rs
  modified:
    - flake.nix
    - .gitignore
    - build/aarch64/boot-9qemu.sh
    - build/aarch64/run-gate.sh
    - build/aarch64/subset-exec.sh
    - build/aarch64/donor-ns-build.sh
    - build/aarch64/donor-vm-build.sh
    - build/aarch64/pre-gate-build.sh
    - build/aarch64/vm-run.sh
    - nix/vm-build.sh
    - build/aarch64/README.md
    - build/aarch64/trace-boot.rc
  deleted:
    - build/aarch64/srv9.py
    - build/aarch64/trace-syscalls.py
    - build/aarch64/gdbpoke9.py
    - build/aarch64/mkfs9.py
    - build/aarch64/drive9.py

key-decisions:
  - "srv9 Twstat name offset corrected to 47 (the Dir wire encoding carries TWO size prefixes); the .py's offset-45 never parsed a name — its Rerror made mv fall back to copy+delete, so IN-05's rename-deletes-target semantics are now genuinely exercised for the first time"
  - "trace-syscalls: wait for the 'term% ' prompt substring after the pid probe, then send immediately — the .py's latent echo/prompt race is closed by construction (tracehello's calibrated 15s spin leaves the quiet window)"
  - "nix package doCheck=false: a mid-TDD RED tree must still give a working devshell; tests run via the plan's explicit cargo test verify"
  - "rsync exclusion for build/harness/target in all four vm-sync scripts (48MB+ wrong-arch cargo output would otherwise ride every guest push)"
  - "fid table per-connection in Rust srv9 (IN-04 — strictly more correct, behavior-identical under the gates' single-active-connection use); IN-05 rename-deletes-target preserved and now live"

requirements-completed: [QUICK-261103-GIQ]

coverage:
  - id: D1
    description: "harness crate: regex engine + srv9/gdbpoke9/mkfs9/console/drive9 subcommands, TDD RED→GREEN"
    requirement: QUICK-261103-GIQ
    verification:
      - kind: unit
        ref: "nix develop -c cargo test --manifest-path build/harness/Cargo.toml (64 tests)"
        status: pass
      - kind: e2e
        ref: "gate (a)+(b)+(d): nix develop -c sh build/aarch64/boot-9qemu.sh → '== PASS: donor kernel booted to an rc shell under TCG'"
        status: pass
    human_judgment: false
  - id: D2
    description: "trace-syscalls subcommand: parser/artifact port + live orchestration (srv9/gdbpoke9 spawned via HARNESS_BIN/current_exe)"
    requirement: QUICK-261103-GIQ
    verification:
      - kind: unit
        ref: "cargo test trace:: (12 parser/writer fixture tests incl. bare-hex-pc + end-of-line ENTRY shapes)"
        status: pass
      - kind: e2e
        ref: "gate (e): TRACE_OUT=/tmp/giq-gate-e-trace.txt nix develop -c harness trace-syscalls → '== PASS: D-14 trace complete'; syscall-name SET == committed 04-01 trace"
        status: pass
    human_judgment: false
  - id: D3
    description: "caller rewiring + docs + five equivalence gates + .py deletion"
    requirement: QUICK-261103-GIQ
    verification:
      - kind: e2e
        ref: "gate (c): guest A/B — mkfs9.py vs harness mkfs9 list stdout identical (89 lines), trees identical (88 files), stderr identical; donor bootfs re-materialized via the disk/mkfs wrapper"
        status: pass
      - kind: e2e
        ref: "run-gate.sh PASS (drive9 --steps 2-row) + subset-exec.sh PASS (22-row, writable-prefix CRUD incl. Twstat rename)"
        status: pass
      - kind: other
        ref: "post-deletion: sh -n ×4, nix flake show evaluates, test ! -e ×5, harness resolves in devshell, drive9 exit-2 smoke"
        status: pass
    human_judgment: false

duration: 81min
completed: 2026-11-03
status: complete
---

# Phase 261103-giq Plan 01: Reimplement the five Python harness tools as one Rust binary Summary

**The five Python harness tools (srv9/drive9/gdbpoke9/mkfs9/trace-syscalls) are now ONE std-only Rust `harness` binary wired into flake.nix + all shell callers, with every D-07 equivalence gate green and the .py files deleted — no python3 dependency remains in the build/aarch64 harness path.**

## Performance

- **Duration:** 81 min
- **Started:** 2026-11-03T17:01:25Z
- **Completed:** 2026-11-03T18:22:47Z
- **Tasks:** 3
- **Files modified:** 28 (14 created/9 modified/5 deleted)

## Accomplishments

- **build/harness crate (std-only, D-02):** a byte-oriented backtracking regex engine (CPS matcher so greedy repeats inside groups backtrack across the boundary — `(\w+)__` works; multiline `^`/`$`, `.` excludes `\n`, `\d\s\w\b` escapes, classes, `*?+`, alternation, captures, ci mode, 2M-step cap per T-GIQ-04); a full 9P2000 server (per-connection fid tables, IN-04); a raw-RSP gdbstub poker; the proto-grammar mkfs9; the shared console drive + drive9 with --steps; and the 1437-line trace-syscalls port. 64 unit/integration tests green via `nix develop -c cargo test`.
- **Flake wiring (D-05/D-06):** `packages.harness` via buildRustPackage with the committed zero-dep Cargo.lock (network-free sandbox build); devshell carries stable nixpkgs cargo/rustc + the harness and exports HARNESS_BIN (nix-store-first, D-04). r9 input, boot-r9/boot-r9-smoke apps, and existing devshell lines untouched.
- **All four callers rewired:** HARNESS_BIN resolution block replaces PYTHON; srv9 startup polls the 'srv9: serving' log line (printed after listen()); the gatedrive9.py and subsetdrive9.py heredocs became 2-row and 22-row steps.tsv tables driving `drive9 --steps`.
- **Five gates green, then deletion:** (c) guest A/B byte-identical (89-line list stdout, 88-file trees, stderr) + donor bootfs re-materialized via the mkfs9 wrapper; (a)(b)(d) boot-9qemu.sh green (srv9 served the whole root, gdbpoke9 woke the redistributor, term% prompt proven); run-gate.sh and subset-exec.sh green (22-step table incl. the writable-prefix mkdir/cp/mv/rm round-trip); (e) hello-path trace PASS with the syscall-name SET equal to the committed 04-01 trace. The five .py files deleted in the final commit; post-deletion sanity all green.
- **The subset gate caught a real porting bug** (exactly what the gates are for): the Twstat stat buffer is doubly size-prefixed on the wire — name at offset 47, not the .py's 45.

## Task Commits

TDD cycles (test → feat per module), then rewiring, fixes found by the gates, and the deletion:

1. `4f0969052` test(261103-giq): harness crate skeleton + regex/srv9 RED tests + flake wiring
2. `941eace9a` feat(261103-giq): std-only backtracking regex engine (GREEN)
3. `a7090e632` feat(261103-giq): srv9 9P2000 file server (GREEN)
4. `f678fb177` test(261103-giq): gdbpoke9/mkfs9/drive9/console RED tests
5. `91b77187b` feat(261103-giq): gdbpoke9 + mkfs9 + console + drive9 (GREEN)
6. `4a60cb7dc` test(261103-giq): trace-syscalls parser RED tests
7. `d93dafa76` feat(261103-giq): trace-syscalls subcommand (GREEN)
8. `941e362d6` feat(261103-giq): rewire build/aarch64 callers to the harness binary (D-04)
9. `5c56fa278` fix(261103-giq): srv9 Twstat name offset — doubly size-prefixed stat buffer
10. `b731dcb01` fix(261103-giq): drop the scratch wstat repro module
11. `680152784` fix(261103-giq): exclude build/harness/target from the lima rsync sets
12. `229871d9e` fix(261103-giq): trace-syscalls: wait for the prompt before the trace command
13. `2af855f0f` fix(261103-giq): trace-syscalls: prompt wait is unanchored + immediate send
14. `c42726256` feat(261103-giq): delete the five Python harness tools (D-07 gates green)

**Plan metadata commit:** handled by the orchestrator (SUMMARY.md not committed by the executor).

## TDD Gate Compliance

| Cycle | RED commit | GREEN commit | Evidence |
|-------|-----------|--------------|----------|
| regex | 4f0969052 | 941eace9a | red-1-regex.json — RED_EVIDENCE_OK (target regex::tests::shell_marker_anchored) |
| srv9 | 4f0969052 (shared skeleton+tests commit) | a7090e632 | same record; srv9 tests failed on assertions via the accept+Rerror stub |
| gdbpoke9/mkfs9/drive9/console | f678fb177 | 91b77187b | red-2-tools.json — RED_EVIDENCE_OK (target gdbpoke9::tests::happy_path_pokes_and_detaches) |
| trace-syscalls parser | 4a60cb7dc | d93dafa76 | red-3-trace.json — RED_EVIDENCE_OK (target trace::tests::hello_region_extracts_first_use_order) |

RED evidence records live in `.planning/quick/261103-giq-reimplement-the-five-python-harness-tool/red-{1,2,3}-*.json` (uncommitted — the orchestrator's docs commit sweeps them).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] srv9 Twstat name offset 45 → 47 (the stat buffer is doubly size-prefixed)**
- **Found during:** Task 3, gate (subset-exec) — `mv` reported OK (`__MV_OK__`) but `ls /tmp/subset` showed the rename never landed.
- **Issue:** the Twstat wire payload is fid(4) + stat-len(2) + **dir-len(2)** + body — the Dir encoding leads with its own u16 length. The .py's offset-45 read 0xffff as the name length and sliced garbage; os.rename then raised ValueError (embedded NUL) → Rerror → mv silently fell back to copy+delete. The 02-07 gate passed that way; the .py's rename path was dead code in practice.
- **Fix:** parse at the real offset (47); rename now genuinely happens, so IN-05's rename-deletes-existing-target semantics are exercised for the first time. Test helpers now build the real double-prefixed donor byte shape. Diagnosed with a new env-gated `SRV9_TRACE=1` per-message trace (kept — off by default).
- **Files modified:** build/harness/src/srv9.rs
- **Verification:** unit suite (64/64) + mini-drive repro (steps 11-14 green) + full subset-exec.sh PASS
- **Committed in:** 5c56fa278 (+ b731dcb01 scratch-test cleanup)

**2. [Rule 1 - Bug] trace-syscalls: close the echo/prompt race before the trace command**
- **Found during:** Task 3, gate (e) first Rust run — parse died "trace region not bracketed": the echoed command arrived as `rterm% c /boot/trace.rc 196` (prompt print interleaved into the echo), so the contiguous start marker `rc /boot/trace.rc` never existed.
- **Issue:** the pid-probe marker (`__GSD_PID__196`) is not a prompt; the .py sent the trace command immediately after it and won by TCG timing luck — a latent race in the original, hit on the first Rust run.
- **Fix:** wait for the `term% ` prompt substring after the pid marker (its print is a single flushed write = rc idle), then send immediately — tracehello's calibrated ~15s pure-compute spin leaves the quiet window. (First attempt used the anchored `^term% $`, which can never fire there — tracehello's prints glue onto the prompt line; corrected to the substring + immediate send.)
- **Files modified:** build/harness/src/trace.rs
- **Verification:** gate (e) rerun green; syscall-name SET == committed 04-01 trace
- **Committed in:** 229871d9e, 2af855f0f

**3. [Rule 2 - Missing critical hygiene] exclude build/harness/target from the lima rsync sets**
- **Found during:** Task 3, gate (c) — the new crate's 48MB+ darwin cargo target dir rode the guest rsync.
- **Fix:** added `--exclude 'build/harness/target'` to the shared exclusion line in donor-vm-build.sh, pre-gate-build.sh, vm-run.sh, nix/vm-build.sh.
- **Verification:** subsequent guest rsyncs carry no target/; guest nix build unaffected (builds from source).
- **Committed in:** 680152784

**4. [Rule 3 - Environment repair] reset the guest-scratch CONF=qemu bootdir to pristine**
- **Found during:** Task 3, subset-exec first run — guest kernel panicked "too many root files" (Nbootfiles=32): the guest-copy CONF had accumulated 39 bootdir rows across every gate since 02-05 (stock 4 + 05-03's 15 + trace 2 + hello 1 + subset 17). Pre-existing guest state, not caused by the port (the same rebake with the Python harness would panic identically); GUEST_REBUILD_BC's prune comment documents exactly this failure mode.
- **Fix:** restored the pristine pinned CONF to the guest copy (guest scratch, never committed; each gate re-adds its own rows idempotently). subset-exec then went green; gate (e)'s staging re-added tracehello/trace.rc as designed.
- **Files modified:** none (guest-only state)
- **Verification:** subset-exec.sh PASS on rerun

---

**Total deviations:** 4 (2 Rule-1 bugs found by the gates, 1 Rule-2 hygiene, 1 Rule-3 environment repair)
**Impact on plan:** the two Rule-1 fixes make the Rust port MORE correct than the originals (a real rename instead of a silent copy+delete fallback; a closed echo race). No scope creep; the .py deletion stayed gated on all five gates passing.

## Verify-command footnote (dirty-tree caveat)

The plan's two literal `head -1` stderr compares (`mkfs9` usage line; `trace-syscalls --help` first line) can only pass on a clean git tree: nix's "warning: Git tree … has uncommitted changes" rides `nix develop`'s stderr ABOVE the harness's first line. This repo carries pre-existing unrelated working-tree edits (rc/bin/Kill, troff/postscript fonts — present before this task, left untouched per the scope boundary). Both assertions were proven equivalent: (a) via the store binary directly (first stderr line byte-exact), and (b) through the devshell with the nix warning filtered (byte-exact line present). On a clean tree the literal commands pass as written.

## Issues Encountered

- A first guest rsync silently mis-delivered (flake.nix updated, Cargo.toml/Cargo.lock missing) and left a stray local `.arvey-build-donor/` duplicate checkout; re-synced with hash verification and removed the stray dir (it was created by this session's rsync; untracked, no content lost). Root cause not fully determined — noted for transparency.
- `nix build` inside the guest needed the committed Cargo.lock present; the resync fixed it. No other guest-side surprises.

## Known Stubs

None — every RED-phase stub was replaced by the GREEN implementation; a stub scan of build/harness/src/ finds no placeholder patterns.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The build/aarch64 harness path is Python-free end to end; the repo's host tooling now matches the r9 convergence story (Rust).
- `harness` is on PATH in every devshell with HARNESS_BIN at a store path; new gates/drives can reuse `drive9 --steps` tables instead of generating per-gate Python heredocs.
- The `SRV9_TRACE=1` knob remains available for future 9P debugging.
- r9 repo untouched (D-09 held — no ../r9 edits).

---
*Phase: 261103-giq*
*Completed: 2026-11-03*

## Self-Check: PASSED

All 10 created crate files exist; all 5 .py files are deleted; all 14 commits (4f0969052 … c42726256) exist on master.
