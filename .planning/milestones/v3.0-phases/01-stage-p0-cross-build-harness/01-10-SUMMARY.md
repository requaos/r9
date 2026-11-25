---
phase: 01-stage-p0-cross-build-harness
plan: 10
subsystem: cross-build-harness
tags: [flake, harvey-input, flake-lock, a13, checks, harvey-harness, harvey-drawterm, ssh-ng-builder, lima, determinate, pin-bump, d-06, d-09, d-23, d-24, harn-07, harn-08]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "pass-1 spike results (S1 builder setup, S2 copy-back FAIL and its remote-builder fix, A9 lazy-path proof), the owner's D-24 decision (both builders, lima primary), and the 01-09 witness driver whose a13 step consumes checks.x86_64-linux.harvey-drawterm"

provides:
  - "inputs.harvey as a normal flake input (never the non-flake form), branchless url, locked at bd7a6982a3e4f465f8ed3da5e0c7290521514ff0 with inputs.harvey.inputs.nixpkgs.follows = nixpkgs (D-09, D-23, D-06)"
  - "checks.<system>.harvey-harness on all three systems and checks.<linux>.harvey-drawterm on aarch64-linux/x86_64-linux — the A13 probes, the x86_64-linux one doubling as the stop-A witness a13 leg"
  - "A13 verdict: CONFIRMED on the Mac (harness builds on aarch64-darwin through the follow; both Linux drawterm drvPaths evaluate); x86_64-linux build pending stop A"
  - "docs/harvey-supply.md: the owner's reversible lima ssh-ng remote-builder registration (steps a-e plus the r9-builder-probe verification), the Determinate native-builder alternative, the pin-bump practice and the session-only local override"

affects: [01-12, 01-13, 01-14, 01-18, 01-28, HARN-07, HARN-08, D-06, D-09, D-10, D-23, D-24]

actuals:
  tokens: 4791
  tasks: 2
  commits: 2

plan_head_before: 940ecce1601251cbb76f211d2bd0e94e8a3e9224
plan_head_after: 56063cf6dfe14da971cab1976d5027609b3264ad

tech-stack:
  added:
    - "Harvey as a flake input (github:requaos/harvey, locked bd7a698) — harvey.packages.<system>.{harness,drawterm} now consumable"
  patterns:
    - "A pinned dependency's url carries no ref, so a version bump is a reviewed one-commit, lock-only change (nix flake lock --override-input)"
    - "Cross-flake follows are exercised by a checks-output probe pair (build the foreign package on the host, evaluate drvPaths cross-system) before any consumer relies on them"
    - "Owner-applied machine configuration is shipped as a doc with one revert per step and a never-substitutable probe derivation as the registration witness"

key-files:
  created:
    - docs/harvey-supply.md
  modified:
    - flake.nix
    - flake.lock

key-decisions:
  - "A13: CONFIRMED on the Mac; x86_64-linux build pending stop A — with inputs.harvey.inputs.nixpkgs.follows = nixpkgs, Harvey's harness built on aarch64-darwin (/nix/store/v1anrpzgyyc0v4can9a25m448q5plgbs-harness-0.1.0) and both drawterm drvPaths evaluated, so the follow stays."
  - "No follow is set on Harvey's own r9 input: Harvey's outputs function takes r9, so an emptied follow would break its evaluation (01-RESEARCH.md Pitfall 8); the nested harvey/r9 lock node at 116bcc23c8052a1728992cdbc7b625143260d683 is expected until the D-10 flip's pin bump (plan 01-28) removes it."
  - "The builder registration doc names the Determinate nix-darwin option exactly as the looked-up docs do (determinateNix.customSettings, which generates /etc/nix/nix.custom.conf) and cites the Nix manual's Remote Builds chapter and builders setting reference for the machine-line format."
  - "The registration verification is a never-substitutable probe derivation (r9-builder-probe-<epoch>): an aarch64-linux build that only the registered builder can service, proving the daemon-side import that fixes S2's copy-back FAIL."

patterns-established:
  - "Branchless input url + lock-only pin bump as the one-commit supply-chain motion"
  - "A checks attribute as the venue probe: harness builds locally, Linux-only packages evaluate cross-system, and the same attribute doubles as the witness step at stop A"
  - "Owner-step documentation shape: exact commands, one Revert (x) per step, the module/option named as the upstream docs name it, and a runtime probe proving the registration"

requirements-completed: [HARN-07, HARN-08]

coverage:
  - id: D1
    description: "Harvey is a normal, locked flake input at bd7a698 with the nixpkgs follow; the lock graph is nested (harvey -> r9 at 116bcc2), not cyclic; the unpushed freeze commit 25d97dc4d never appears in flake.lock"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "jq .nodes.harvey.locked.rev = bd7a6982a3e4f465f8ed3da5e0c7290521514ff0; jq nested harvey/r9 rev = 116bcc23c8052a1728992cdbc7b625143260d683; harvey/nixpkgs follows [nixpkgs]; grep -c 25d97dc4d flake.lock = 0; grep -c 'flake = false' flake.nix = 0; nix flake show --all-systems exit 0 (commit 6dfc1f4)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A13 verdict recorded: harness builds on aarch64-darwin through the follow and both Linux drawterm drvPaths evaluate; the x86_64-linux drawterm BUILD leg is not claimed here — it is the stop-A witness's a13 step"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix build --no-link --print-out-paths .#checks.aarch64-darwin.harvey-harness -> /nix/store/v1anrpzgyyc0v4can9a25m448q5plgbs-harness-0.1.0; nix eval .#checks.x86_64-linux.harvey-drawterm.drvPath -> /nix/store/kgc1fwq2j6lwm1a0h01x23k2f6nvlxwy-drawterm-19.06.drv; .#checks.aarch64-linux.harvey-drawterm.drvPath -> /nix/store/44a4gi1yh0f6xpbifgk66ldl9bih081c-drawterm-19.06.drv (commit 6dfc1f4)"
        status: pass
    human_judgment: false
  - id: D3
    description: "docs/harvey-supply.md gives the owner exact, copy-pasteable, reversible steps to register the lima VM as an ssh-ng remote builder (primary), documents the Determinate native builder as the alternative, and covers pin bumps plus the session-only local override"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "DOC-KEYS-OK (all seven literal keys); revert count 7 >= 5; 4 nix.dev + 6 docs.determinate.systems URL citations; /etc/nix ls output and nix.custom.conf sha256 identical before/after (commit 56063cf)"
        status: pass
    human_judgment: true
    rationale: "The greps prove the doc's content; the registration steps themselves are owner-applied at stop A and unproven until the r9-builder-probe derivation runs against the registered builder — the executor is forbidden from applying them (HARN-07 prohibition)."
  - id: D4
    description: "No kernel bytes changed and no Mac configuration changed: both kernel ELF sha256 values match the 01-08 baseline before and after, and /etc/nix is untouched"
    verification:
      - kind: other
        ref: "x86_64 q35churnshell sha256 50b01c9a03077e4ae71f5a505b2d9f6df8e93ad79973c536c324c5276a18929f and aarch64 virtshell 76e6e153f578b7022da74570802729e51d3105b9e3ffab0c31ef12917fec56d2 identical at plan start and after the last commit; HARVEY-UNTOUCHED guard printed; /etc/nix diff empty"
        status: pass
    human_judgment: false

duration: 6min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 10: Harvey as a flake input, the A13 check, and the supply documentation Summary

**r9's flake now takes Harvey as a normal input locked at the pushed legacy-final commit bd7a698 with the nixpkgs follow CONFIRMED by the A13 probe (harness built on the Mac, both Linux drawterm drvPaths evaluate), and docs/harvey-supply.md gives the owner the reversible lima ssh-ng remote-builder registration, the Determinate alternative, the pin-bump practice and the session-only override.**

## Performance

- **Duration:** ~6 min of executor time (04:58:17Z to 05:04:41Z)
- **Started:** 2026-11-23T04:58:17Z
- **Completed:** 2026-11-23T05:04:41Z
- **Tasks:** 2 of 2
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments

- `flake.nix` gains `inputs.harvey.url = "github:requaos/harvey";` (branchless, so pin bumps stay lock-only per D-06) with `inputs.harvey.inputs.nixpkgs.follows = "nixpkgs";`, and `harvey` joins the outputs argument set. No follow is set on Harvey's own `r9` input — Harvey's outputs function takes `r9`, so an emptied follow would break its evaluation (Pitfall 8).
- A `checks` output provides the A13 probes: `harvey-harness` on every system and `harvey-drawterm` where `isLinux` (Harvey's drawterm is a Linux-only package). `checks.x86_64-linux.harvey-drawterm` is exactly the attribute the 01-09 witness driver's `step_a13` builds at stop A.
- `flake.lock` pins harvey at `bd7a6982a3e4f465f8ed3da5e0c7290521514ff0` via `nix flake lock --override-input`. The lock graph is nested, not cyclic: root -> harvey (bd7a698) -> r9 (`116bcc23c8052a1728992cdbc7b625143260d683`, an older revision) plus harvey's nixpkgs followed to r9's. The unpushed freeze commit `25d97dc4d` appears zero times in the lock.
- **A13 verdict: CONFIRMED on the Mac; x86_64-linux build pending stop A.** `nix build --no-link --print-out-paths .#checks.aarch64-darwin.harvey-harness` produced `/nix/store/v1anrpzgyyc0v4can9a25m448q5plgbs-harness-0.1.0`; `nix eval --raw` of `.#checks.x86_64-linux.harvey-drawterm.drvPath` and `.#checks.aarch64-linux.harvey-drawterm.drvPath` printed `/nix/store/kgc1fwq2j6lwm1a0h01x23k2f6nvlxwy-drawterm-19.06.drv` and `/nix/store/44a4gi1yh0f6xpbifgk66ldl9bih081c-drawterm-19.06.drv`. The follow stays.
- `nix flake show --all-systems` evaluates without error (exit 0) and lists the new checks on all three systems (no drawterm on aarch64-darwin, by design).
- `docs/harvey-supply.md` writes the stop-A owner runbook: the lima VM `harvey` as an `ssh-ng://r9-lima-builder` remote builder in five steps (port pin 60022, root-owned `/etc/nix/r9-lima-builder` key, root Host alias, the `determinateNix.customSettings` `builders`/`builders-use-substitutes` lines through the nix-darwin Determinate module, keep the guest trusted-users line), each with its exact revert; the never-substitutable `r9-builder-probe-<epoch>` verification that proves the daemon-side import fixing S2's copy-back FAIL; the Determinate native builder (`determinate-nixd auth login`) as the alternative with its unmeasured case-collision caveat; the pushed-commit-first pin-bump rule (`git ls-remote … | grep <sha>`, D-23); and the session-only `--override-input harvey path:… --no-write-lock-file` local override (D-06).

## Task Commits

Each task was committed atomically:

1. **Task 1: Add inputs.harvey (normal input, follow nixpkgs), lock it to bd7a698, add the A13 checks** - `6dfc1f4` (build)
2. **Task 2: Write docs/harvey-supply.md (Mac builder registration, pin bumps, overrides)** - `56063cf` (docs)

**Plan metadata:** the `docs(01-10): complete …` commit that carries this SUMMARY (REQUIREMENTS.md unchanged — see Issues Encountered).

## Files Created/Modified

- `flake.nix` (modified) — inputs.harvey with the nixpkgs follow; harvey in the outputs set; the `checks` output with harvey-harness (all systems) and harvey-drawterm (Linux)
- `flake.lock` (modified) — harvey locked at bd7a698, nested harvey/r9 node at 116bcc2, harvey/nixpkgs following root's nixpkgs
- `docs/harvey-supply.md` (created) — the builder registration runbook, alternative, pin bumps, overrides, venues, and cited sources

## Decisions Made

- **A13 is CONFIRMED on the Mac, and the follow stays.** Both probes passed with `inputs.harvey.inputs.nixpkgs.follows = "nixpkgs"` in place, so the removal path the plan offered was not taken. The x86_64-linux drawterm *build* is deliberately not claimed: it is the stop-A witness's a13 leg (plan 01-12), and the plan scopes this plan to evaluation on the Mac.
- **The nested `harvey/r9` node is kept as-is.** It is an older-revision node (116bcc2), not a cycle, and plan 01-28's pin bump removes it after the Harvey-side flip (D-10).
- **The doc names upstream options exactly as their docs do.** The Determinate nix-darwin page names `determinateNix.customSettings` as the attribute that writes `/etc/nix/nix.custom.conf` (module 3.15.2+; this Mac runs 3.21.0), and the machine-line field order was verified against the Nix manual's `builders` reference before writing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Reworded a comment that tripped the plan's own guard**
- **Found during:** Task 1's acceptance loop, before the first commit
- **Issue:** The acceptance criterion `grep -c 'flake = false' flake.nix == 0` is a guard mutation against declaring Harvey a non-flake input. My first draft's comment quoted the literal form ("never `flake = false`"), which made the guard print 1 — the comment itself defeated the check meant to catch the real defect.
- **Fix:** Reworded the comment to "never the spike flakes' non-flake input form"; the guard prints 0 and remains truthful against a real regression.
- **Files modified:** flake.nix
- **Verification:** `grep -c 'flake = false' flake.nix` = 0; `grep -c 'inputs.harvey.url = "github:requaos/harvey";' flake.nix` = 1
- **Committed in:** `6dfc1f4`

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** None on scope or design; the fix keeps the plan's guard meaningful.

## Issues Encountered

- **`requirements-completed` is copied verbatim from the plan, but REQUIREMENTS.md was not ticked.** `requirements.ready-ids` reports both `HARN-07` and `HARN-08` blocked because sibling plans in this phase also declare them and have no SUMMARY yet — the same shared-ID-gate behaviour plans 01-08 and 01-09 recorded. The later plans' completions release the ticks.
- The unlock's `nix flake lock`/`nix flake show` warnings ("Git tree … has uncommitted changes") were expected mid-task noise while the input edits awaited their commit; the locked rev and narHash are unaffected.

## User Setup Required

None generated by this plan. The owner-applied builder registration is intentionally documentation-only here (HARN-07 prohibition); `docs/harvey-supply.md` is the artifact stop A consumes.

## Known Stubs

None. The doc's attribute names for the per-arch tree outputs are deliberately forward-referenced to plans 01-13/01-14 ("the attribute names the variables resolve to are filled in by the tree-derivation plans"), which is the plan's own staging, and the witness `a13` step resolves against the check this plan ships.

## Threat Flags

None beyond the plan's threat model. The implemented surface matches its mitigations: T-01-34 (harvey locked by rev + narHash; branchless url; unpushed-commit grep guard recorded in the doc), T-01-35 (root-owned key under /etc/nix, loopback-only alias, documented reverts), T-01-36 (`StrictHostKeyChecking accept-new` scoped to the 127.0.0.1:60022 lima forward), T-01-SC (no package installs; new lock nodes are pinned narHashes).

## Next Phase Readiness

- Plan 01-12 (stop A): the owner registers the builder from `docs/harvey-supply.md`, then runs the witness (`R9_WITNESS_STEPS="rio control a13"`); its `a13` step builds `checks.x86_64-linux.harvey-drawterm`, which now exists.
- Plans 01-13/01-14 can define the `harvey-tree-<arch>` derivations against the locked input; the A13 follow question is settled, so they inherit a single-nixpkgs graph.
- Plan 01-28's pin bump removes the nested `harvey/r9` node; the bump procedure and its pushed-commit proof are already documented.
- Kernel bytes unchanged (both ELFs identical to the 01-08 baseline before and after); Harvey untouched (HEAD `25d97dc4d`, status hash `041d2929…`); no push; no Mac configuration touched (`/etc/nix` ls output and `nix.custom.conf` sha256 identical before/after Task 2).

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Files on disk: `flake.nix`, `flake.lock`, `docs/harvey-supply.md`, and this SUMMARY all FOUND.
- Task commits `6dfc1f4` and `56063cf` are ancestors of HEAD; `git rev-list --count 940ecce..HEAD` = 2 (measured against the plan ledger).
- Task 1 acceptance re-run: `flake = false` count 0; harvey url count 1; `.nodes.harvey.locked.rev` = `bd7a6982a3e4f465f8ed3da5e0c7290521514ff0`; nested harvey/r9 = `116bcc23c8052a1728992cdbc7b625143260d683`; `25d97dc4d` count 0; harness built on aarch64-darwin; both Linux drawterm drvPaths printed; `nix flake show --all-systems` exit 0; `cargo xtask test`/`clippy`/`check` green.
- Task 2 acceptance re-run: DOC-KEYS-OK; `revert` count 7 ≥ 5; URLs cited (nix.dev ×4, docs.determinate.systems ×6); `/etc/nix` ls output and `nix.custom.conf` sha256 identical before/after.
- Plan verification: HARVEY-UNTOUCHED (HEAD `25d97dc4d`, status hash `041d2929…`); kernel ELF sha256 `50b01c9a…`/`76e6e153…` identical before the first commit and after the last.
