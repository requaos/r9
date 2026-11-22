---
phase: 16-host-net-modes-networking-exit-gate
plan: 06
subsystem: netdev-infra
tags: [net-smoke, slirp, hostfwd, emitter-argv, expect, ci, github-actions, actionlint, x86_64, hermetic, tcg]
duration: 15 min
completed: 2026-11-11
requires:
  - "16-04-complete the R9_NETDEV_EMITTER flake export at the advanced pin (r9 d1c4849 — the gate list already carries netdev-args)"
  - "16-05-complete the R9_NETDEV_ARGV split-spawn driver mechanism (netsoak-r9.expect's {*}$netdev_argv shape)"
provides:
  - "the composed net gate re-keyed onto the emitter's slirp argv: nix/smoke-r9-net-x64.sh contract-checks R9_NETDEV_EMITTER (-f/-x), captures the argv once, exports R9_NETDEV_ARGV; the expect driver splits the tokens — ZERO netdev literals left in the driver (venue FIXED slirp per D-02, the D-04 hostfwd pair now flows from the same pinned TOML data the boot app consumes — D-05's front-end widening, ONE emission path)"
  - "two new host-side observation stages (hostfwd-17010-listening, hostfwd-5670-listening): exec ss -tln (netstat fallback) on the HOST while the booted guest runs, asserting both loopback hostfwd entries — positive evidence, loopback only; qemu's eager-bind abort is the backstop"
  - "the r9-shell.yml net-smoke-x64 job (ubuntu-latest, 90m, env R9_ACCEL=tcg nix run .#boot-r9-net-smoke-x64) — HNET-01's hosted-CI leg, both actions pinned to full SHAs per the 06-08 rule; tap/soak legs named primary-host-only by construction (D-13)"
  - "the gate green TWICE locally on KVM with both captures retained (evidence/16-06-netsmoke-a.log / -b.log, NOT-L2-TRUTH venue labels intact)"
affects:
  - "16-08 (the phase exit gate re-runs this gate; evidence consolidation)"
  - "Phases 18/19 (the observed 127.0.0.1:17010 and 127.0.0.1:5670 listeners are the drawterm/auth remap targets the D-04 pair exists for)"
tech-stack:
  added: []
  patterns:
    - "host-side observation stages inside an expect driver: exec ss -tln (netstat fallback) between pty stages — exec never touches the spawn's pty, the boot output accumulates in expect's buffer and the next stage consumes it; port tokens anchored `( |$)' so 5670 can never match 56702"
    - "the 16-05 argv-split spawn mechanism generalized to the slirp venue: the caller captures the emitter once (before any build spend), the driver's required-env block refuses to run without R9_NETDEV_ARGV — no hardcoded netdev fallback can ever exist"
key-files:
  created:
    - .planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-06-netsmoke-a.log
    - .planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-06-netsmoke-b.log
  modified:
    - nix/smoke-r9-net-x64.sh
    - build/x86_64/smoke-r9-net.expect
    - flake.nix
    - .github/workflows/r9-shell.yml
key-decisions:
  - "The gate's venue stays FIXED slirp (D-02) — the re-key changes the argv's SOURCE (the emitter's pinned TOML, hostfwd included), never the venue; the NOT-L2-TRUTH honesty labels and their stage asserts carry untouched"
  - "The hostfwd observation stages sit between the banner and paqfs stages (the plan's placement): qemu binds hostfwd eagerly at netdev init, so the listeners provably exist by banner time, and the positive observation is logged while the guest is live"
  - "iproute2 joined the net-smoke app's runtimeInputs (Rule 3): the driver's `exec ss' must not depend on ambient PATH — the 16-03 doctor app's precedent for exactly this tool"
  - "The CI job block is compact (no - name: labels) so the run line sits inside the plan verify's `grep -A6 net-smoke-x64:' window — same actions, same pins, same structure as the sibling net-smoke job"
  - "PENDING FIRST PUSH: the hosted-CI observation of net-smoke-x64 lands with the phase's push (the standing convention — the orchestrator/user owns the push)"
patterns-established:
  - "Loopback-only listener proof: assert `127.0.0.1:<port>' presence in the HOST listening table, never reachability from a non-loopback address; a longer-port neighbor must not satisfy the match (anchored regexp, negative-control tested)"
requirements-completed: [HNET-01]
actuals:
  tokens: 3973    # chars/4 over the four source files' diff 71640cdf..4bac09784 (evidence logs excluded — gate output, not authored; estimate was 45000 — wiring, not invention, the 16-04 pattern again)
  tasks: 2
  commits: 2      # measured: git rev-list --count 71640cdf70a712006ee3f411b1adea6184550e44..4bac097840396435692b0e59b7f3c5b6c96a2193
plan_head_before: 71640cdf70a712006ee3f411b1adea6184550e44
plan_head_after: 4bac097840396435692b0e59b7f3c5b6c96a2193
status: complete
coverage:
  - id: D1
    description: "The composed net gate consumes the emitter's slirp argv (no netdev literal left in the driver) and observes both hostfwd loopback listeners host-side while the guest runs"
    requirement: HNET-01
    verification:
      - kind: other
        ref: "EMITTER-REKEY-OK (grep R9_NETDEV_EMITTER in the sh, R9_NETDEV_ARGV in the expect driver, `netdev user' count in the driver == 0)"
        status: pass
      - kind: other
        ref: "Two full nix run .#boot-r9-net-smoke-x64 runs on KVM: exit-a=0 and exit-b=0, both consoles carry `stage ok — hostfwd-17010-listening' and `stage ok — hostfwd-5670-listening', the spawn echo shows the pinned r9-netdev-args-d1c4849 argv with both hostfwd rules"
        status: pass
      - kind: other
        ref: "GREEN-TWICE-OK: the fresh run-B capture carries NET-SMOKE + netip: NETIP-DONE; both retained captures (evidence/16-06-netsmoke-a.log / -b.log) carry `netip: NOT-L2-TRUTH slirp venue' (2 occurrences each — static + dhcp legs)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The r9-shell.yml net-smoke-x64 CI job (TCG, pinned actions, tap/soak legs named primary-host-only), authored and linted no-push"
    requirement: HNET-01
    verification:
      - kind: other
        ref: "nix run nixpkgs#actionlint -- .github/workflows/r9-shell.yml → LINT-OK; `net-smoke-x64:' present; R9_ACCEL=tcg inside the -A6 window; actions/checkout@11d5960a… count == 2 (increased by exactly 1); git show 4bac09784 = one file, +23/-0 (only the new job block)"
        status: pass
    human_judgment: false
  - id: D3
    description: "HNET-01's green-in-hosted-CI observation (the net-smoke-x64 job actually running green on a hosted runner)"
    requirement: HNET-01
    verification: []
    human_judgment: true
    rationale: "PENDING FIRST PUSH — the hosted observation lands with the phase's push (the user/orchestrator owns the push, the standing convention since 16-04's pin advance); nothing can execute the hosted leg from this checkout pre-push"
---

# Phase 16 Plan 06: Net Gate on the Emitter Argv + the Hosted CI Leg Summary

**The composed net gate's qemu spawn now consumes the pinned r9 emitter's slirp argv (D-04 hostfwd pair included, one emission path everywhere), observes both loopback hostfwd listeners host-side mid-boot, is green twice on KVM with captures retained — and a pinned-action TCG job (net-smoke-x64) carries HNET-01's hosted-CI leg, authored, linted, PENDING FIRST PUSH.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-11-11T13:58:36Z
- **Completed:** 2026-11-11T14:13:27Z
- **Tasks:** 2
- **Files modified:** 6 (4 source files + 2 retained evidence captures)

## Accomplishments

- The gate re-key is live end-to-end: the flake app exports `R9_NETDEV_EMITTER` (the pinned `r9-netdev-args-d1c4849` store path), the gate script captures the slirp argv once and exports `R9_NETDEV_ARGV`, and the expect driver's spawn block splits the tokens — `grep -c 'netdev user'` on the driver is 0 (was 2)
- Two new host-side observation stages make the hostfwd evidence positive: `hostfwd-17010-listening` and `hostfwd-5670-listening` read the host's listening table (`exec ss -tln`, netstat fallback) between the banner and paqfs stages and assert the loopback entries while the guest runs — loopback only, never non-loopback reachability (the plan's prohibition honored by construction)
- The observation procs were unit-tested in isolation before the gate run: positive match, the 56702-vs-5670 anchored non-match (negative control), the end-of-string edge, and the empty-table fail-loud arm
- The gate is green TWICE locally on KVM (47 stage-ok lines each run), both captures retained with the NOT-L2-TRUTH venue labels intact (the honesty label survived the re-key)
- `net-smoke-x64` joins r9-shell.yml: pinned checkout SHA, pinned nix-installer SHA, explicit `R9_ACCEL=tcg` (hosted runners are KVM-less — the no-silent-downgrade discipline), 90m TCG-scale timeout, actionlint-clean

## Task Commits

Each task was committed atomically:

1. **Task 1: the net gate rides the emitter argv + hostfwd listener observation** - `f01450794` (feat)
2. **Task 2: the r9-shell.yml net-smoke-x64 CI job (no-push validation)** - `4bac09784` (test)

**Plan metadata:** (docs commit follows this summary)

## Files Created/Modified

- `nix/smoke-r9-net-x64.sh` — R9_NETDEV_EMITTER contract check (the -f/-x idiom), the one-shot slirp argv capture exported as R9_NETDEV_ARGV (before any build spend), the VENUE + NETDEV header note (D-02/D-05)
- `build/x86_64/smoke-r9-net.expect` — required-env gains R9_NETDEV_ARGV; the spawn block splits the emitter tokens (`{*}$netdev_argv`); two hostfwd observation stages after the banner; header/env-contract comments re-keyed (zero netdev literals)
- `flake.nix` — netSmokeR9X64Pkg exports R9_NETDEV_EMITTER; iproute2 joins its runtimeInputs (the driver's `exec ss')
- `.github/workflows/r9-shell.yml` — the new net-smoke-x64 job (purely additive, +23/-0)
- `evidence/16-06-netsmoke-a.log`, `evidence/16-06-netsmoke-b.log` — the two green captures (completion marker + venue labels present)

## Decisions Made

- **Stage placement after the banner (per plan):** qemu binds hostfwd eagerly at netdev init, so the listeners provably exist once the banner prints; the observation runs while the guest is live, and the pty buffer simply carries the paqfs/version output past the exec calls
- **iproute2 in runtimeInputs (Rule 3):** the driver's `exec ss` must come from the app's own closure, not ambient PATH (the 16-03 doctor's precedent); the netstat fallback stays in the driver as the plan words it
- **Compact CI job shape:** the run line must sit within the plan verify's `grep -A6 'net-smoke-x64:'` window, so the job drops the sibling job's cosmetic `- name:` labels — identical actions, pins, and structure otherwise

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] iproute2 missing from the app's runtimeInputs**
- **Found during:** Task 1 (flake edit)
- **Issue:** the plan's flake.nix change names only the extraHeader export, but the driver's new `exec ss -tln` observation needs `ss` on the app's PATH — relying on ambient PATH would break the app's hermetic contract (and the 16-03 doctor already established iproute2 as the ss provider)
- **Fix:** `pkgs.iproute2` added to netSmokeR9X64Pkg's runtimeInputs, with the comment block explaining why
- **Files modified:** flake.nix
- **Verification:** both green gate runs executed the stages through the app wrapper (`stage ok — hostfwd-…` in both consoles)
- **Committed in:** f01450794

**2. [Rule 1 - Bug] Tcl array-parse hazard in the port regexp (caught pre-run)**
- **Found during:** Task 1 (driver authoring)
- **Issue:** `"…:$port( |\$)"` makes Tcl parse `$port( |\$)` as an ARRAY element reference (`variable isn't array` at runtime) — `$port` directly followed by `(` is the array syntax
- **Fix:** `${port}` brace-delimited substitution; the isolated proc unit test then proved positive match, the 56702-vs-5670 anchored non-match, the end-of-string edge, and the empty-table arm
- **Files modified:** build/x86_64/smoke-r9-net.expect
- **Verification:** /tmp/hostfwd-stage-test.expect + /tmp/hostfwd-edge-test.expect (all arms), then two green end-to-end runs
- **Committed in:** f01450794

**3. [Rule 3 - Cleanup] Gate-build byproducts left untracked in the source tree**
- **Found during:** Task 1 (post-run git status)
- **Issue:** the gate's tier build regenerates yacc outputs and binaries in-tree (sys/src/cmd/{awk,rc,grep,units} — awkgram.c, proctab.c, units.c, x.tab.h, 6.out, unix.maketab), which the tree deliberately does not track
- **Fix:** removed the regenerable files after the second green run (restores the pre-run tree; no .gitignore churn mid-task)
- **Files modified:** none (untracked files removed)
- **Verification:** git status clean of non-evidence untracked files before the task commit
- **Committed in:** n/a (cleanup)

---

**Total deviations:** 3 auto-fixed (1x Rule 1, 2x Rule 3)
**Impact on plan:** none on scope — all plan verifies green; the fixes are the wiring the plan's wording implied but did not name

## Issues Encountered

None — both gate runs green on the first boot after the re-key (the 14-09 pattern holding again: the re-key touched only the spawn's argv source, not the venue's behavior).

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## PENDING FIRST PUSH

The hosted-CI observation of the new `net-smoke-x64` job (HNET-01's green-in-hosted-CI leg) lands with the phase's push — the standing convention (16-04's pin advance carried the same marker). Local proof is complete: the job is authored, actionlint-clean, and runs the same app that is green twice on this host.

## Next Phase Readiness

- The composed net gate is emitter-backed and hostfwd-observing — 16-08's exit gate and evidence consolidation consume it as-is
- Phases 18/19's drawterm/auth legs inherit the observed 127.0.0.1:17010 and 127.0.0.1:5670 listeners (the D-04 remap's whole purpose)
- Remaining in phase 16: plans 07-08 (per ROADMAP); no blockers from this plan

## Self-Check: PASSED

All 7 created/modified files exist; both task commits (f01450794, 4bac09784) are ancestors of HEAD.

---
*Phase: 16-host-net-modes-networking-exit-gate*
*Completed: 2026-11-11*
