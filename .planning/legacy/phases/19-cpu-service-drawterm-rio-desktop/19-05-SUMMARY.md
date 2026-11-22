---
phase: 19-cpu-service-drawterm-rio-desktop
plan: "05"
subsystem: infra (FOD/pin integrity chain, INT-06)
tags: [r9, fod, pin-advance, lock-only, hosted-ci, gates-re-green, int-06, two-derivation-proof]

requires:
  - phase: 19-cpu-service-drawterm-rio-desktop/19-01
    provides: the bootconf-seed/append-seam kernel bytes (r9 e491ed5) this FOD pass carries
  - phase: 19-cpu-service-drawterm-rio-desktop/19-02
    provides: the AcceptPool donor TCP shape + q35 tick_check wiring (r9 df9ada8)
  - phase: 19-cpu-service-drawterm-rio-desktop/19-03
    provides: the #! exec pre-pass + WSTAT/FWSTAT/REMOVE arms (r9 f691914)
  - phase: 19-cpu-service-drawterm-rio-desktop/19-04
    provides: devssl #D (r9 07889e2) — the last kernel bytes before the pass
  - phase: 18-in-guest-p9sk1-auth-domain/18-08
    provides: the two-derivation FOD choreography this plan repeats verbatim
provides:
  - harvey flake.lock r9 pin advanced 9103522 -> 0a4b853 (lock-only, never bundled)
  - the boot-iso FOD hash re-wired over ALL 19-01..19-04 kernel bytes together (one pass, three-way-agreed)
  - a GREEN hosted CI run at the pushed r9 HEAD (run 43663178433 — the first green boot-test-x86-64 leg ever, via the R9_HARVEY_DIR workflow fix)
  - every existing composed gate re-greened at the advanced pin (auth green twice)
affects: [19-07 (the seventh contract consumes kernel-x64 from this pin), 19-08, 19-09, 19-10, harvey r9-shell.yml consumers]

actuals:
  tokens: 4400    # chars/4 over realized changes: r9 diff 3405c + harvey flake.lock 647c + this SUMMARY ~13500c (estimate was 36000)
  tasks: 2
  commits: 1      # MEASURED: git rev-list --count d1e950a0e..HEAD (harvey; the docs commit rides atop) + r9 2 (9d0b225 + 0a4b853, its own repo, pushed)
plan_head_before: d1e950a0ecacad65bba2188baba25fce41e48f6a
plan_head_after: f52f9cd73aea09d628a63afc09c8ca2ad85c4411   # the lock-only pin commit; this SUMMARY's metadata commit sits directly atop

tech-stack:
  added: []        # zero packages (T-19-SC honored); one CI action-step addition (actions/checkout for the harvey tree)
  patterns:
    - "Hosted-witness repair as part of the pin choreography: the CI leg the discipline observes must actually be green-able — a never-green leg makes the INT-06 observation vacuous, so the precondition gap (R9_HARVEY_DIR) is fixed push-before-pin in the same push the observation rides"
    - "Cross-repo CI checkout: the ureg-check gate's harvey dependency is one 2008-stable header, so a default-branch public checkout of requaos/harvey satisfies the pin without a submodules dance"

key-files:
  created: []     # this SUMMARY is the only new file
  modified:
    - ../r9/flake.nix                        # the FOD re-wire (its own commit 0a4b853, pushed) — outputHash + the 19-05 lineage entry
    - ../r9/.github/workflows/xtask.yml      # the boot-test CI leg fix (its own commit 9d0b225, pushed) — harvey checkout + R9_HARVEY_DIR + mtools
    - flake.lock                             # r9 9103522 -> 0a4b853 (lock-only commit f52f9cd73)

key-decisions:
  - "The one FOD pass covers ALL FOUR kernel plans together (19-01 bootconf/append seam + 19-02 AcceptPool/tick_check + 19-03 #!/WSTAT/FWSTAT/REMOVE + 19-04 devssl — 9103522..07889e2): a re-wire over a partial tree would re-stale at the next plan's bytes, so the pass waits for the last kernel commit exactly as the plan's must-have pins"
  - "The hosted CI observation gates the pin advance (the must-have's own order): run 43663178433 went green BEFORE the lock-only commit landed — push-before-pin extended to push-before-advance, the run URL recorded here as the provenance artifact"
  "The CI-leg fix is push-before-pin with the re-wire: both r9 commits (9d0b225 the workflow fix, 0a4b853 the re-wire) rode ONE push, so the observed green run and the pinned HEAD are the same tree — the re-wire commit itself still touches flake.nix ONLY (the acceptance criterion's file-list check holds commit-by-commit)"
  - "Services gate honestly NOT run: it stays blocked by the pre-existing #68 dns-stage kernel trap (A/B-proven pre-existing at earlier pins; owner a future phase) — stated here, never silently omitted"
  - "Harvey origin observed 8 commits behind local master (docs + 19-03's 83ae39392): NOT pushed — no plan authority over the harvey remote, and the r9 CI fix is unaffected (its only harvey dependency is the 2008-stable ureg.h); recorded as an observation for the user"

patterns-established:
  - "FOD getFlake venue: pure eval refuses unlocked path: refs ('cannot call getFlake on unlocked flake reference') — the non-FOD lib.bootIso realization rides git+file:///<repo>?rev=<sha> with --impure; the lib function itself stays the documented pure surface"

requirements-completed: [INT-06]   # plan frontmatter verbatim; the shared-ID gate verb governs the actual checkbox flips

coverage:
  - id: D1
    description: "The phase's ONE FOD pass over all 19-01..19-04 kernel bytes: the full battery at the final kernel HEAD, the two-derivation byte-identity proof, the re-wire, the push"
    requirement: INT-06
    verification:
      - kind: other
        ref: "BATTERY-OK (nix develop -c cargo xtask check + clippy, port 160, x86_64 bins 103) + boot legs q35cpu/q35auth/q35shell/q35net KVM PASS (cpurc: RAN, X64-NET-DONE in the serial logs) + q35net TCG PASS; derivation A (plain 07889e2) ISO /nix/store/9k49lr82…-r9-boot-iso-07889e2 vs derivation B (marker clone fbc5ac2, FRESH r9-kernel-x64-fbc5ac2.drv) ISO /nix/store/dbhy4hf…-r9-boot-iso-fbc5ac2: cmp clean, file sha256 186e223f8232b76ede5ba0c2e945f18afb6a6584af84da855c71ea1d7ff38802 both, NAR sha256-Xlth4PpyXtHmkzqHhrSyFbDIVCzGWeVnfd0zLoKYFl8= both == the FOD's own got-value (base16 5e5b61e0fa725ed1e6933a8786b4b215b0c8542cc659e5677ddd332e8298165f); wired at r9 0a4b853 (flake.nix ONLY), nix build .#boot-iso + --rebuild green, re-verified from the committed pushed rev; PUSHED-CLEAN (origin/main..HEAD empty, tree clean)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The fresh hosted CI run at the pushed r9 HEAD, green and recorded BEFORE the pin-advance commit landed (the never-green boot-test leg repaired in the same push)"
    requirement: INT-06
    verification:
      - kind: integration
        ref: "GitHub Actions run 43663178433 at head 0a4b853, https://github.com/requaos/r9/actions/runs/43663178433 — conclusion SUCCESS, all six jobs green (lint, tests, build-x86_64/aarch64/riscv64, boot-test-x86-64); the boot-test step ran the three TCG profiles for real (step window 01:28:28-01:29:45Z: fresh kernel build + q35/q35lowmcfg/q35net boots) — the job's first green since its 13-06 introduction (35 consecutive failures, every run red at the R9_HARVEY_DIR precondition)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The pin advance (its OWN lock-only commit) + every existing composed gate re-greened at the advanced pin, auth green twice"
    requirement: INT-06
    verification:
      - kind: integration
        ref: "PIN-LOCK-ONLY (git diff HEAD~1 --name-only is exactly flake.lock) + LOCK-EQUALS-R9-ORIGIN-MAIN (flake.lock r9 rev 0a4b853c90333ceee3101c76f5f8321978a1ae5e == r9 origin/main); nix run .#boot-r9-auth-x64 twice: AUTH-GATE-GREEN-A + AUTH-GATE-GREEN-B (ALL STAGES PASS both, AUTH-DONE present, zero AUTH-FAIL, 10 AUTH-OK verdicts in leg B's capture, build/x86_64/auth-r9.log); boot-r9-net-smoke-x64 exit 0 ALL STAGES PASS (NET-SMOKE-GREEN); boot-r9-smoke-x64 + boot-r9-tier1-x64 exit 0 (SMOKE-TIER1-GREEN); boot-r9-churn-x64 exit 0 ALL STAGES PASS (84 B/exit within the 1772 window)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The services gate NOT claimed: blocked by the pre-existing #68 dns-stage kernel trap, stated rather than silently omitted"
    verification:
      - kind: other
        ref: "boot-r9-services-x64 deliberately not run this plan (the plan's own instruction): the #68 dns-stage trap is A/B-proven pre-existing at earlier pins (17-04's record), kernel-byte-identical across this pin advance, and owned by a future phase — this row is the honest non-claim"
        status: pass
    human_judgment: false

duration: 42 min
completed: 2026-11-15
status: complete
commits_note: "harvey plan_head_before=d1e950a0e plan_head_after=f52f9cd73; r9 07889e2..0a4b853 (both pushed, == origin/main)"
---

# Phase 19 Plan 05: The FOD Pass + Pin Advance + Hosted CI + Gates Re-Green Summary

**The phase's pin choreography closed end-to-end: the ONE FOD pass proved all 19-01..19-04 kernel bytes byte-identical across two independent derivations (three-way NAR agreement, re-wired and pushed at r9 0a4b853), the never-green hosted CI boot-test leg was repaired and observed GREEN at the pushed HEAD before the lock-only pin advance landed (run 43663178433 — the leg's first green since 13-06), and every existing composed gate re-greened at the advanced pin with auth green twice**

## Performance

- **Duration:** 42 min | **Started:** 2026-11-15T01:15:56Z | **Completed:** 2026-11-15T01:58:15Z | **Tasks:** 2/2 | **Files modified:** 3 (2 r9, 1 harvey)

## The Two-Derivation Proof (the record, 18-08's style)

| Derivation | Tree state | Kernel build | ISO realization |
|---|---|---|---|
| A (plain) | 07889e2 (r9 main, the 19-04 push) | same drv as the FOD probe (cached) | non-FOD `lib.bootIso` → /nix/store/9k49lr82ygqmkyby93mvr3v8a1pfa5gf-r9-boot-iso-07889e2 |
| B (independence) | fbc5ac2 (marker clone: 07889e2 + a kernel-neutral README marker commit, discarded) | FRESH (r9-kernel-x64-fbc5ac2.drv built) | non-FOD `lib.bootIso` → /nix/store/dbhy4hfsv59anf9lch0cp0a049z9i41v-r9-boot-iso-fbc5ac2 |
| The FOD itself | 07889e2, stale wired hash (18-08's 8uL7mR…) | same | the mismatch report's got-value |

- **Byte identity:** `cmp` clean; file sha256 `186e223f8232b76ede5ba0c2e945f18afb6a6584af84da855c71ea1d7ff38802` on both ISOs
- **Three-way NAR agreement:** A == B == the FOD's own got-value == `sha256-Xlth4PpyXtHmkzqHhrSyFbDIVCzGWeVnfd0zLoKYFl8=` (base16 `5e5b61e0fa725ed1e6933a8786b4b215b0c8542cc659e5677ddd332e8298165f`)
- Wired at r9 `0a4b853` (its own `fix(19-05)` commit, flake.nix ONLY, with the lineage comment naming all four plans' bytes); `nix build .#boot-iso` + `--rebuild` green at the wired value, re-verified from the committed pushed rev (`git+file://…?rev=0a4b853` realization green)
- Harvey pin advanced `9103522 -> 0a4b853` in `f52f9cd73` — lock-only (PIN-LOCK-ONLY verified commit-to-commit), LOCK-EQUALS-R9-ORIGIN-MAIN

## The Hosted CI Record (the INT-06 witness)

- **Run 43663178433** — https://github.com/requaos/r9/actions/runs/43663178433 — head `0a4b853`, conclusion **SUCCESS**, all six jobs green (lint / tests / build-x86_64 / build-aarch64 / build-riscv64 / boot-test-x86-64)
- The boot-test step genuinely ran the three TCG profiles (step window 01:28:28Z–01:29:45Z: fresh kernel build + `q35` / `q35lowmcfg` / `q35net` boots under `--accel tcg --cpu qemu64`) — the **first green boot-test-x86-64 since the leg's 13-06 introduction** (35 consecutive failures before, every one dying ~30s in at the R9_HARVEY_DIR precondition the workflow never satisfied)
- Ordering per the must-have: the run went green at 01:29:47Z, the pin-advance commit landed after — a fresh hosted CI run IS green on the pushed r9 HEAD before the pin advance, and the run is recorded here

## Accomplishments

- The integrity chain is whole at the phase's pin: every composed gate now consumes the bootconf-seed + AcceptPool + tick_check + #! + WSTAT/FWSTAT/REMOVE + devssl kernel bytes through flake.lock 0a4b853, proven by six green gate runs at the pin (auth twice)
- The hosted CI witness — required by INT-06 and never before actually green on the boot-test leg — is now a real, repeatable green signal (the workflow fix rides the same push the observation validated)
- The FOD pass followed the 18-08 choreography exactly: got-value first (the stale-hash mismatch report), two independent derivations through the non-FOD `lib.bootIso`, byte identity + like-for-like NAR three-way agreement, re-wire, rebuild verification, push

## Task Commits

Each task was committed atomically (cross-repo: the re-wire lands in r9 `main`, push-before-pin; the pin advance is harvey's, lock-only):

1. **Task 1: battery + two-derivation FOD proof + re-wire + push** — r9 `9d0b225` (fix; the CI-leg repair, the Rule-3 deviation below) + r9 `0a4b853` (fix; the FOD re-wire, flake.nix only)
2. **Task 2: pin advance + hosted CI + gates re-green** — harvey `f52f9cd73` (chore; the lock-only pin advance)

**Plan metadata:** (docs: this SUMMARY + STATE/ROADMAP — the commit follows directly)

## Files Created/Modified

- `../r9/flake.nix` — outputHash → `sha256-Xlth4PpyXtHmkzqHhrSyFbDIVCzGWeVnfd0zLoKYFl8=` + the 19-05 lineage entry (all four plans' bytes named)
- `../r9/.github/workflows/xtask.yml` — the boot-test job's harvey checkout (`requaos/harvey`, public, default branch) + `R9_HARVEY_DIR` step env + `mtools` in the apt set
- `flake.lock` — the r9 node's rev/narHash/lastModified advanced (3 lines), nothing else

## Decisions Made

See key-decisions. Additionally: the B battery used the marker-clone idiom (never-before-used version string → no cache path can serve the kernel), and derivation A intentionally shares the FOD probe's cached kernel drv (18-08's "cached at the same drv" posture — B is the fresh-build independence leg).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The hosted CI boot-test leg had NEVER been green — the workflow was missing its R9_HARVEY_DIR precondition**
- **Found during:** Task 2 preparation (the CI-observation precondition check, before any pin advance)
- **Issue:** Every `cargo xtask boot-test` invocation resolves `$R9_HARVEY_DIR/amd64/include/ureg.h` BEFORE any boot (the cross-repo Ureg-table pin, 14-01 D-02b); r9's xtask.yml never set the variable and never checked out the Harvey tree. Consequence: 35 consecutive red runs since the leg's 13-06 introduction (verified across the full run history — every run 2026-11-08..2026-11-15 failed the boot-test job while lint/tests/builds stayed green), each dying ~30s into the step. The plan's must-have ("a fresh hosted CI run is green on the pushed r9 HEAD before the pin-advance commit lands") was unsatisfiable without repairing the leg.
- **Fix:** The workflow patch (r9 `9d0b225`, its own commit, push-before-pin in the same push as the re-wire): a `requaos/harvey` default-branch checkout (public repo, anonymous read; the header is 2008-stable so ref choice cannot skew the pin) + `R9_HARVEY_DIR: ${{ github.workspace }}/harvey` in the step env + `mtools` in the apt set (hybrid-ISO insurance for grub-mkrescue's mformat arm). The three CI profiles (q35/q35lowmcfg/q35net) are kernel-only — the header is the leg's only Harvey dependency, verified in the xtask source.
- **Files modified:** ../r9/.github/workflows/xtask.yml
- **Verification:** run 43663178433 green end-to-end at the pushed HEAD (the observed witness itself); the leg's step timings show a real kernel build + three boots
- **Commit:** r9 9d0b225

### Verify adaptations (not deviations)

- The battery/boot-test commands ride `nix develop -c` (the host has no bare cargo — the 19-02 venue precedent; the plan's bare `cargo xtask …` wording assumes the devshell).
- The non-FOD `lib.bootIso` proofs carry `--impure` with `git+file://…?rev=` refs: pure eval refuses unlocked `path:` flake references ("cannot call getFlake on unlocked flake reference"). The lib function itself remains the documented pure surface; nothing about the ISO realization changed.

**Total deviations:** 1 auto-fixed (Rule 3 blocking). **Impact:** the fix is the minimal provisioning repair that makes the plan's own INT-06 witness satisfiable — no gate logic, no kernel bytes, no flake surface touched; the re-wire commit's file list stays exactly flake.nix.

## Issues Encountered

- The initial `git fetch origin` in harvey failed once with a transient DNS error (`Could not resolve hostname github.com` over SSH); the same operation succeeded minutes later, and the r9 push went through unaffected.
- Harvey's origin was found 8 commits behind local master (docs commits + 19-03's `83ae39392`). Not pushed — this plan has no authority over the harvey remote, and nothing hosted consumes those bytes at this pin (r9's CI fix reads only the 2008-stable ureg.h). Flagged here for the user.
- The boot-test step completing in 77s initially looked suspicious for "three TCG boots + kernel build"; the step-level timings (fresh build ~60s + boots) and the driver's own ALL-STAGE verdicts — the same driver that reds correctly on bad input across 13 phases — confirm a genuine green.

## Known Stubs

None - no stubs, placeholders, or unwired data paths were introduced. (The services gate's non-run is a plan-specified honest exclusion, not a stub — see D4.)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 19-07 (the seventh mkimage contract) and everything downstream consumes kernel-x64 from the 0a4b853 pin — all six composed gate greens at this pin are its entry evidence
- The hosted CI witness is now green-able and green: future pin advances can re-observe it without re-diagnosing the leg
- The FOD hash re-stales on the next kernel-byte commit (the standing drift class) — the NEXT FOD pass repeats this choreography (one pass over the phase's remaining kernel bytes, if any)

## Self-Check: PASSED

- Modified files exist on disk: ../r9/flake.nix, ../r9/.github/workflows/xtask.yml, flake.lock FOUND
- Task commits exist and are pushed: r9 9d0b225, 0a4b853 (== origin/main, PUSHED-CLEAN at verify time) FOUND; harvey f52f9cd73 FOUND (ancestor of HEAD)
- The pin-advance commit's file list is exactly flake.lock (PIN-LOCK-ONLY re-run at close-out); flake.lock r9 rev == r9 origin/main (LOCK-EQUALS-R9-ORIGIN-MAIN)
- All plan <verify> lines re-confirmed: BATTERY-OK, PUSHED-CLEAN, PIN-LOCK-ONLY, AUTH-GATE-GREEN-A, NET-SMOKE-GREEN, SMOKE-TIER1-GREEN, AUTH-GATE-GREEN-B (the auth twice), plus the churn gate's ALL STAGES PASS; CI run 43663178433 SUCCESS recorded above

---
*Phase: 19-cpu-service-drawterm-rio-desktop*
*Completed: 2026-11-15*
