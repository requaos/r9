---
phase: 01-stage-p0-cross-build-harness
plan: 18
subsystem: cross-build-harness
tags: [xtask, gate, drawterm, rio, expect, xvfb, xdotool, flake-app, kvm, d-07, d-09, d-11, d-12, d-13, d-14, d-28, harn-01, harn-02, harn-08]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-16's cpu/auth images and the per-bake authdomain (target/images/x86_64/cpu.paq + cpu.creds); 01-17's full amd64 tree; 01-12/01-32's stop-A2 rio verdict (the X11 feed + the D-28 pair)"
provides:
  - "nix/gates/drawterm-gate-x64.sh: the moved drawterm gate app script (tree + r9 assembler, creds by 0600 file path, R9_GATE_OUT, no silent downgrade)"
  - "gates/x86_64/drawterm-gate.expect: the moved driver with session, rio, control-noG and force-no-rio modes, GATE_SERIAL_CHECKS, pid-based teardown plus a run-identity sweep"
  - "flake.nix: Harvey's mkScriptApp helper, apps.x86_64-linux.drawterm-gate-x64 and the Linux devshell gate tools"
  - "xtask/src/gate.rs: GateStep — the quick tier (image:cpu, q35cpu kernel/ISO, the rio + control legs, the tree-unmodified proof) and the full tier (app evals, nix run, full.tsv)"
affects: [01-21, 01-22, 01-23, 01-24, 01-30, 01-31, HARN-01, HARN-02, HARN-08, D-11, D-12, D-13, D-14, D-28]

actuals:
  tokens: 19633
  tasks: 2
  commits: 2

plan_head_before: f454f05556b82a90c7403eddff8ff77bb00a5770
plan_head_after: ed52b475bc5e6e61641eee02b3ff80a0513a8efe

tech-stack:
  added:
    - "r9 flake app apps.x86_64-linux.drawterm-gate-x64 (writeShellApplication via the copied mkScriptApp helper)"
    - "cargo xtask gate (quick + full tiers)"
  patterns:
    - "A moved gate app keeps Harvey's env-contract header but every package reference becomes r9's own (`self.packages.${system}`), the tree comes from R9_HARVEY_DIR_AMD64, and the bake becomes `cargo xtask image`"
    - "The gate's password travels only as a 0600 file path (R9_DEMO_PWFILE); the script copies the creds line into it, the driver types it with xdotool --file, and the EXIT/cleanup path removes it"
    - "The driver owns its own Xvfb (-displayfd, -nolisten tcp) so `cargo xtask gate` hands it nothing but the ISO and the env contract; the stop-A XTEST feed is reused unchanged"
    - "Every exit path reaps qemu/drawterm/Xvfb by recorded pid, then sweeps the user's qemu/Xvfb/drawterm processes for the run tag or scratch identity; the argv0 tag is only an extra key"
    - "The quick gate proves HARN-02 after the legs on pass AND fail: `nix store verify --no-trust` for a store tree, a recomputed FNV-1a tree digest for an override"
    - "Venue discipline is a precondition, not a runtime fallback: non-Linux or missing grub-mkrescue is GATE-VENUE naming the Linux devshell, and kvm without a writable /dev/kvm is GATE-VENUE kvm-unavailable"

key-files:
  created:
    - nix/gates/drawterm-gate-x64.sh
    - gates/x86_64/drawterm-gate.expect
    - xtask/src/gate.rs
  modified:
    - flake.nix
    - xtask/src/main.rs

key-decisions:
  - "The driver, not the shell app, starts Xvfb: `cargo xtask gate` invokes the expect driver directly, so the rio leg's private display (-displayfd, -nolisten tcp) has to live where the feed and teardown do."
  - "Session mode keeps the creds user (norbert) and the Harvey -G assertions byte-for-byte; the rio family logs in as GATE_DT_USER=glenda, the only staged home profile, exactly as the stop-A2 mechanism did (RIO-DERISK LOGIN-USER)."
  - "force-no-rio runs the rio pair assertion over a -G session and exits non-zero with GATE-FAIL rio-not-serving — the same marker a genuine rio failure prints, so the failing direction is exercised by the same code path."
  - "The driver has no skip-rio switch and the quick gate runs the control leg unconditionally: the pair is the only PASS condition and the control discriminates it."
  - "The serial checks are parsed and validated in xtask before any spawn, then joined newline-wise for the driver (one <cmd>=<marker> per line)."
  - "The full tier's app lists are the future moved app names; until plans 01-21/01-23/01-24/01-30/01-31 register them, --full fails with GATE-FAIL full-apps-missing naming each missing app instead of running a partial set."

patterns-established:
  - "Moved gate: HARVEY=$R9_HARVEY_DIR_AMD64 with the HARVEY-TREE stamp, build/bake -> `cargo xtask image`, outputs under R9_GATE_OUT, R9_SKIP_BUILD deleted"
  - "Driver modes are one chassis (boot ladder, hostfwd observation, cpurc ladder, post-passes) with a mode branch at the session and the serial assertion"
  - "Gate failures are named markers on stdout (GATE-FAIL ...), stage prose on stderr, and `cargo xtask gate` returns the leg's marker in its Err"

requirements-completed: [HARN-01, HARN-02, HARN-08]

coverage:
  - id: D1
    description: "The drawterm gate script and driver live in r9 and read only the read-only tree and r9 scratch; session mode keeps the Harvey-hosted assertions"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- nix/gates/drawterm-gate-x64.sh (clean, rc 0)"
        status: pass
      - kind: unit
        ref: "echo 'set f [open gates/x86_64/drawterm-gate.expect]; puts [info complete [read $f]]' | tclsh (prints 1)"
        status: pass
      - kind: other
        ref: "grep -vE '^[[:space:]]*#' nix/gates/drawterm-gate-x64.sh | grep -cE 'stage-build\\.sh|subset-build\\.sh|mkimage\\.sh|R9_SKIP_BUILD' (0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The flake registers apps.x86_64-linux.drawterm-gate-x64 with r9's supply and puts the gate tools in the Linux devshell"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix eval --raw .#apps.x86_64-linux.drawterm-gate-x64.program -> /nix/store/h4pbwbqzy3pqq8zk5j43ki5vxd6d7z96-drawterm-gate-x64/bin/drawterm-gate-x64"
        status: pass
    human_judgment: false
  - id: D3
    description: "cargo xtask gate resolves the venue and accel without a silent downgrade, parses the serial checks, digests an override tree, and carries the full-tier app lists"
    requirement: HARN-01
    verification:
      - kind: unit
        ref: "xtask/src/gate.rs#accel_prefers_the_flag_then_the_env_then_kvm, #accel_never_downgrades_kvm_to_tcg, #quick_venue_refuses_the_mac_and_names_the_linux_devshell, #parse_serial_check_splits_on_the_first_equals, #tree_digest_is_stable_and_changes_with_file_bytes, #full_tier_lists_are_complete (6 passed)"
        status: pass
      - kind: other
        ref: "nix develop -c cargo xtask gate --arch x86-64 on the Mac -> GATE-VENUE not-linux (MAC-VENUE-REFUSED)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The quick tier boots the r9 kernel + r9-assembled cpu image on q35 and exits 0 only on the rio pair plus the clean control (the D-28 golden invariant), twice in a row on the owner's KVM host; force-no-rio exits non-zero with GATE-FAIL rio-not-serving"
    requirement: HARN-01
    verification: []
    human_judgment: true
    rationale: "x86_64 ISO composition and the KVM boot are Linux-only; the Mac cannot realise or witness the run. The KVM witness runs are queued for stop B (plan 01-22) per D-13/D-14."
  - id: D5
    description: "Every quick run proves the Harvey tree byte-for-byte unchanged afterwards (nix store verify for a store path, a content digest for an override), failing with GATE-FAIL harvey-tree-modified"
    requirement: HARN-02
    verification:
      - kind: unit
        ref: "xtask/src/gate.rs#tree_digest_is_stable_and_changes_with_file_bytes"
        status: pass
    human_judgment: true
    rationale: "The unit test proves the digest's stability/sensitivity; the store-verify leg needs a real store tree and a completed quick run, which only the Linux KVM venue can produce (stop B)."
  - id: D6
    description: "cargo xtask gate --full drives the moved gate apps in sequence and records full.tsv"
    requirement: HARN-01
    verification: []
    human_judgment: true
    rationale: "The moved apps do not exist in r9's flake until plans 01-21/01-23/01-24/01-30/01-31; --full is proven to fail loudly naming each missing app, and its end-to-end drive is due at the full-tier stop."

duration: 5 min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 18: The quick gate — `cargo xtask gate` with the rio leg, and the drawterm gate moved into r9

**`cargo xtask gate` is now r9's golden-invariant command: it assembles image:cpu, builds the q35cpu kernel, boots q35, proves the D-28 rio pair plus the clean -G control, and verifies the Harvey tree is unmodified — with the drawterm script and driver living in r9 and exposed as the `drawterm-gate-x64` flake app.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-11-23T11:10:43Z
- **Completed:** 2026-11-23T11:15:17Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Moved Harvey's `nix/drawterm-gate-x64.sh` to `nix/gates/drawterm-gate-x64.sh` and rewired every build/bake step onto r9: `HARVEY=$R9_HARVEY_DIR_AMD64` with the `HARVEY-TREE` stamp, `cargo xtask image --arch x86-64 --image cpu` in place of the in-checkout tier build and bake, creds read from `target/images/x86_64/cpu.creds` and passed only as a 0600 file path, outputs under `R9_GATE_OUT`, and `R9_SKIP_BUILD` deleted.
- Moved the driver to `gates/x86_64/drawterm-gate.expect` with four modes: `session` (the unchanged -G assertions), `rio` (non -G drawterm under a private Xvfb, the stop-A XTEST password feed, the serial `ls /srv` pair assertion, the documented manual-start fallback, `GATE-PASS rio-serving`), `control-noG` (the unchanged -G control, `GATE-PASS control-no-rio`) and `force-no-rio` (the pair assertion over -G, ending in `GATE-FAIL rio-not-serving`), plus optional `GATE_SERIAL_CHECKS` and pid-based teardown with a run-identity sweep and capture redaction.
- Registered `apps.x86_64-linux.drawterm-gate-x64` in the r9 flake with Harvey's `mkScriptApp` helper and r9's own supply (kernel-x64, iso-builder, netdev-args, harvey-tree-amd64, mkpaqfs, authdomain-gen, Harvey's drawterm), and added expect/Xvfb/xdotool/procps/iproute2/drawterm to the Linux devshell.
- Added `xtask/src/gate.rs` with the quick tier (tree identity, image:cpu, q35cpu kernel through `BuildStep`, ISO through `build_x86_64_iso`, the rio and control-noG legs, the tree-unmodified proof, `GATE PASS quick x86_64 run=<id>`) and the full tier (app existence evals, `nix run` in order, `target/gate/<run-id>/full.tsv`), plus the venue/accel discipline (`GATE-VENUE`, `GATE-VENUE kvm-unavailable`) and six unit tests.

## Task Commits

Each task was committed atomically:

1. **Task 1: Move the drawterm gate into r9, rewire it, add the rio/control/force modes; register the app** — `c0b641a` (feat)
2. **Task 2: `cargo xtask gate` — quick and full tiers, venue and accel discipline, tree-unmodified check** — `ed52b47` (feat)

_Note: Task 2 is `tdd="true"`; the six behaviour tests were written with the implementation and landed in one feat commit (see Process Deviations)._

## Files Created/Modified

- `nix/gates/drawterm-gate-x64.sh` (created) — the moved gate app script: tree/assembler rewiring, 0600 creds file, `R9_GATE_OUT`, accel discipline, EXIT-trap sweep.
- `gates/x86_64/drawterm-gate.expect` (created) — the moved driver with `session`/`rio`/`control-noG`/`force-no-rio`, Xvfb + XTEST feed, the D-28 pair, `GATE_SERIAL_CHECKS`, teardown and redaction.
- `flake.nix` (modified) — `mkScriptApp`, `apps.x86_64-linux.drawterm-gate-x64`, Linux devshell gate tools.
- `xtask/src/gate.rs` (created) — `GateStep`: quick and full tiers, venue/accel, `tree_digest`, serial-check parsing, the full-tier app lists.
- `xtask/src/main.rs` (modified) — `mod gate`, the `gate` subcommand, its dispatch arm, `BuildStep::for_gate`.

## Decisions Made

- The expect driver starts its own Xvfb (private display, `-nolisten tcp`) because `cargo xtask gate` invokes the driver directly; the shell app and the xtask quick tier therefore share one venue implementation.
- `session` mode keeps the creds user (`norbert`) and the Harvey assertions; the rio family logs in as `glenda` (`GATE_DT_USER`), the only staged home profile, matching the stop-A2 proof.
- `force-no-rio` reuses the genuine rio assertion over a -G session so the failing direction is exercised by the same code path, and exits non-zero with `GATE-FAIL rio-not-serving`.
- The full tier names the future moved apps and fails loudly with `GATE-FAIL full-apps-missing` until plans 01-21/01-23/01-24/01-30/01-31 register them, rather than running a partial set.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] clap value name with `=` does not parse in the `arg!` macro**
- **Found during:** Task 2 (the first `cargo test` build)
- **Issue:** `clap::arg!(--"serial-check" <cmd=marker> ...)` rejects `=` in a value name (`no rules expected '='`).
- **Fix:** use `<pair>` as the value name and document `<cmd>=<marker>` in the help text and `parse_serial_check`.
- **Files modified:** xtask/src/main.rs
- **Verification:** `cargo test -p xtask gate::` compiles and passes.
- **Committed in:** ed52b47 (Task 2 commit)

**2. [Rule 1 - Bug] mixed Option types in the tree-unmodified check**
- **Found during:** Task 2 (first test build)
- **Issue:** one `if` arm produced `Option<Box<dyn Error>>` (store verify) and the other `Option<String>` (digest), so inference failed.
- **Fix:** normalise both arms to `Option<String>` (`store_verify(...).err().map(|e| e.to_string())`).
- **Files modified:** xtask/src/gate.rs
- **Verification:** `cargo test -p xtask gate::` passes; clippy/check green.
- **Committed in:** ed52b47 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs), 1 process deviation.
**Impact on plan:** The auto-fixes were compile-time corrections inside the new code. The TDD process deviation is recorded rather than hidden; the behaviour tests cover every `<behavior>` bullet and pass.

### Process Deviations

**1. [TDD] Task 2's RED commit was not separated**
- **Found during:** Task 2 (recorded at SUMMARY time)
- **Issue:** Task 2 carries `tdd="true"` (with a `<behavior>` block), but its six behaviour tests and the implementation landed together in `ed52b47` (`feat`), so no `test(01-18)` RED commit with failing-test evidence exists. The plan frontmatter is `type: execute`, so the plan-level TDD gate does not apply; the task-level sequence was still not followed.
- **Fix:** none retroactively — rewriting the commit sequence would fabricate RED evidence. The tests were run after the fact and all six pass; the omission is recorded here and in `.planning/WINDOWS.md`.
- **Files modified:** none
- **Verification:** `cargo test -p xtask gate::` reports 6 passed.
- **Committed in:** ed52b47 (the single feat commit)

## Issues Encountered

- The devshell eval needs new files tracked: `git add -N` was used for the two new files so `nix eval` could see them; both were committed in Task 1.
- The Mac cannot run the quick gate (no Linux, no grub, no KVM): it refuses with `GATE-VENUE not-linux` as designed. The KVM witness runs stay queued for stop B (plan 01-22).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- HARN-01's command exists with its rio leg, control and failing direction; the KVM green-twice runs are due at stop B (plan 01-22).
- The D-12 checks are green: `cargo xtask test`, `clippy` and `check`; the q35churnshell kernel ELF sha256 equals the 01-08 baseline (`50b01c9a…`); Harvey is untouched (HEAD `25d97dc4d`, status hash `041d2929…`).
- The moved app names in the full tier are the contract plans 01-21/01-23/01-24/01-30/01-31 must satisfy.

---
*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files exist: `nix/gates/drawterm-gate-x64.sh`, `gates/x86_64/drawterm-gate.expect`, `xtask/src/gate.rs`, `01-18-SUMMARY.md`.
- Modified files exist: `flake.nix`, `xtask/src/main.rs`.
- Commits are ancestors of HEAD: `c0b641a` (Task 1), `ed52b47` (Task 2).
- Task acceptance criteria re-ran: `Command::new("gate")` = 1; `harvey-tree-modified` = 3 (>=1); no skip/no-rio-check = 0; `GATE-PASS rio-serving` = 3; `GATE-PASS control-no-rio` = 3; `exp_pid` = 2; `R9_DEMO_PW[^F]` total = 0.
- Task verifies re-ran: shellcheck clean; tclsh info complete = 1; app eval = /nix/store path; Mac venue refused with GATE-VENUE; `cargo xtask test`/`clippy`/`check` rc 0; kernel sha256 equals the 01-08 baseline.
