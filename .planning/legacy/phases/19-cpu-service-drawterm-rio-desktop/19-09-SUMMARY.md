---
phase: 19-cpu-service-drawterm-rio-desktop
plan: "09"
subsystem: infra (the ONE-command full-system mode + the full-chain smoke)
tags: [boot-r9-x64, --system, cpu-service, smoke-r9, expect, hostfwd, cpurc, §15, bootconf-append, flake-app, INT-05]
requirements: [INT-05]

# Dependency graph
requires:
  - phase: 19-cpu-service-drawterm-rio-desktop/19-07
    provides: mkimage §15's bootfs-cpu.paq + the cpu-bringup stage vocabulary + the append-seam consumer shape
  - phase: 19-cpu-service-drawterm-rio-desktop/19-08
    provides: the concurrency-corrected kernel at the df4ad78 pin + ETHER-OK (the hostfwd reception seam in cpurc's ladder)
  - phase: 16-host-net-modes-networking-exit-gate
    provides: the committed D-04 hostfwd pair (127.0.0.1:17010/5670) as pinned TOML data through the r9 emitter
provides:
  - boot-r9-x64's --system=cpu|terminal mode (INT-05/the OQ-4 settlement) — the ONE command brings up the full system (net + auth + cpu listener) off §15's image with the bootconf append; terminal (the default) keeps the chain path byte-untouched, proven by the standing smoke
  - the full-chain smoke leg — the standing boot-r9-smoke-x64 gate now asserts the whole bring-up in ONE composed gate (chain leg untouched + cpu leg: bootconf, hostfwd observation, the ladder through LISTEN-17010/DONE, zero FAIL, the DESK-02 desktop-absent proof), green twice consecutively (three total green full runs)
  - the README's ONE-command/full-system/full-chain sections (the honest-venue scope boundary vs 19-08's drawterm gate)
affects: [19-10 (the clean-checkout proof rides --system=cpu and the extended smoke as the standing gate), any future boot-r9-x64 consumer (the --system surface is now the documented full-system entry)]

actuals:
  tokens: 12417     # chars/4 over the realized production diff (49,669 chars across nix/boot-r9-x64.sh, nix/smoke-r9-x64.sh, build/x86_64/smoke-r9.expect, build/x86_64/README.md, flake.nix; evidence logs excluded)
  tasks: 3
  commits: 3        # MEASURED: git rev-list --count 7a8f2842d..HEAD
plan_head_before: 7a8f2842d07b400a6bd3072ae1dcf7880d3b0e74
plan_head_after: f58ff7f81cae9f37134295f2b62d94e34ef5ec51

tech-stack:
  added: []          # zero packages (T-19-SC honored); iproute2/procps are flake runtimeInputs of the existing smoke app, not new deps of the tree
  patterns:
    - "Full-system mode as a flag-over-env-over-default chain on the boot app (--system > R9_SYSTEM > terminal, the --net/R9_NET precedent verbatim) with single-accepted-form parse and usage errors naming the valid set — never a silent downgrade"
    - "The two-leg composed gate: one expect driver, two spawns in one run — the existing leg's stages byte-untouched, the new leg APPed after its post-passes; the first guest killed before the second spawns (one live guest at a time; the new leg owns the hostfwd wire)"
    - "cpu-mode bake assert via the tee'd bake log grep (the drawterm-gate discipline: under a tee the pipeline rc is tee's, so the §15 PASS-line grep IS the failure detection); terminal's bake stays rc-direct — byte-identical"

key-files:
  created:
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-09-smoke-chain-a.log
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-09-smoke-cpu-a.log
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-09-smoke-chain-b.log
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-09-smoke-cpu-b.log
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-09-smoke-a-stages.txt
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-09-smoke-b-stages.txt
  modified:
    - nix/boot-r9-x64.sh            # --system=cpu|terminal: the parse, the image-selection seam, the append compose, the system banner
    - nix/smoke-r9-x64.sh           # the cpu-chain leg's pipeline: tier build + §15 assert + the append-bearing cpu ISO + the two-capture gate
    - build/x86_64/smoke-r9.expect  # the cpu stages + hostfwd observation + the cpurc:FAIL fast-fail arm + the desktop-absent post-pass
    - flake.nix                     # the smoke app's R9_NETDEV_EMITTER export + iproute2/procps runtimeInputs
    - build/x86_64/README.md        # the ONE-command + full-chain sections

key-decisions:
  - "--system lands as flag > R9_SYSTEM env > terminal default — the boot app's own --net/R9_NET machine-contract precedent carried to the new surface; absent flag AND env = terminal, the exact prior behavior (the default-path smoke green is the proof)"
  - "cpu mode additionally runs subset-build.sh in step 1: mkimage §15 names it as the producer of cmd/cpu/aux:listen/sed/ipconfig, so without it the ONE command is two commands off a clean checkout — the roadmap's promise required it (deviation Rule 2)"
  - "The cpu leg's ladder asserts ETHER-OK explicitly — STRONGER than cpu-bringup.expect's 19-07 vocabulary (which predates the 19-08 ether leg and merely tolerates the marker in its buffer); one vocabulary, two drivers, the standing gate carries today's full ladder"
  - "cpu-mode's bake assert greps §15's PASS line from a tee'd bake log: under a tee the pipeline rc is tee's (a failed bake would be masked), so the grep is the teeth — the drawterm-gate discipline; terminal's bake invocation stays rc-direct and byte-identical"
  - "The smoke app's emitter/iproute2/procps additions ride the EXISTING hasAttr-gated x64 block — the frozen gate list stays byte-untouched (netdev-args has gated the whole block since 16-04, so no pin-advance choreography is triggered)"

patterns-established:
  - "Two-leg composed gate: kill the first spawn's process group at the leg boundary, re-spawn, re-log_file — the per-leg capture keeps each boot's whole-file post-passes independent evidence"

requirements-completed: [INT-05]

coverage:
  - id: D1
    description: "boot-r9-x64's full-system mode (Task 1): --system=cpu|terminal in the single-accepted-form discipline; cpu selects §15's bootfs-cpu.paq with the append compose; terminal byte-untouched; usage errors name both modes; the banner names the venue"
    requirement: INT-05
    verification:
      - kind: other
        ref: "MODE-WIRED + SYNTAX-OK (the plan's verify legs) + the negative controls: --system=foo -> 'want terminal|cpu', duplicate --system -> one-mode-per-boot error, --system cpu (space form) -> unknown-argument error, R9_SYSTEM=cpu honored past the parse"
        status: pass
      - kind: e2e
        ref: "DEFAULT-PATH-UNTOUCHED: nix run .#boot-r9-smoke-x64 exit 0, ALL STAGES PASS on the unmodified smoke at the new boot app (the terminal default rides the byte-identical chain path)"
        status: pass
      - kind: e2e
        ref: "The live --system=cpu boot: the whole cpurc ladder green on one interactive run (PAIR-OK, ETHER-OK, KEYFS-OK, AUTHSRV-LISTEN, FACTOTUM-OK, LISTEN-17010, DONE; the bootconf seed line exactly once; system banner lines present) — /tmp/19-09-task1-cpu-boot.log"
        status: pass
    human_judgment: false
  - id: D2
    description: "The full-chain smoke leg (Task 2): the standing gate asserts net + auth + cpu bring-up in ONE composed gate — the chain leg untouched, the cpu leg carried from cpu-bringup's vocabulary, green twice consecutively"
    requirement: INT-05
    verification:
      - kind: e2e
        ref: "CHAIN-SMOKE-A-GREEN: nix run .#boot-r9-smoke-x64 exit 0, ALL 35 STAGES PASS (both legs; evidence/19-09-smoke-{chain,cpu}-a.log + -a-stages.txt)"
        status: pass
      - kind: e2e
        ref: "CHAIN-SMOKE-GREEN-TWICE: two more consecutive full runs exit 0 (evidence/19-09-smoke-{chain,cpu}-b.log; run A vs B captures DISTINCT — genuinely independent per-bake domains)"
        status: pass
      - kind: other
        ref: "VOCAB-CARRIED (LISTEN-17010 in the smoke expect) + the desktop-absent post-pass present (latin1.8.font count asserted 0 — DESK-02's backstop in the standing gate) + driver bash -n + expect info-complete + the flake app set evals (21 apps)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The README's ONE-command and full-chain sections (Task 3): the --system flag reference, the full-system recipe, the smoke-leg scope note, the aarch64-untouched note"
    requirement: INT-05
    verification:
      - kind: other
        ref: "README-ONECMD-OK (the plan's verify: --system=cpu recipe + ONE-command phrasing present) + the scope-boundary section at build/x86_64/README.md"
        status: pass
    human_judgment: false

duration: 24 min
completed: 2026-11-15
status: complete
---

# Phase 19 Plan 09: The ONE-command full-system mode + the full-chain smoke Summary

**boot-r9-x64 gained --system=cpu|terminal (INT-05/the OQ-4 settlement: the ONE command now boots net + auth + the cpu listener together off §15's image with the bootconf append, terminal default byte-untouched), and the standing smoke became a two-leg composed gate asserting the whole chain — the §15 boot, the hostfwd pair observed host-side, the cpurc ladder through LISTEN-17010/DONE, zero FAIL, and the desktop provably absent from boot — green twice consecutively (three total green full runs)**

## Performance

- **Duration:** 24 min
- **Started:** 2026-11-15T14:01:08Z
- **Completed:** 2026-11-15T14:25:20Z
- **Tasks:** 3/3
- **Files modified:** 5 production + 6 evidence

## Accomplishments

- **Task 1 (7a2e32c3b):** `--system` joins boot-r9-x64's arg parse in the --net single-accepted-form discipline (= form only, duplicate = error, unknown mode = usage error naming exactly terminal|cpu; R9_SYSTEM env mirrors R9_NET). The image-selection seam: cpu selects out/bootfs-cpu.paq — asserted non-empty post-bake WITH §15's PASS manifest line via the tee'd-bake-log grep (the drawterm-gate discipline; the grep is the teeth under the tee-masked pipeline rc) — and composes the ISO WITH the bootconf append (init=/amd64/init -c, 19-01's seam); terminal keeps the chain paq with the bake invocation and compose argv byte-identical. cpu mode additionally runs the tier build (subset-build.sh, §15's named producer) so the full system is ONE command off a clean checkout. The banner/echo lines name the selected venue. Verified three ways: greps + syntax, the unmodified existing smoke green (default untouched), and a live --system=cpu interactive boot green through the whole ladder — plus four parse negative controls.
- **Task 2 (2f49d6eb3):** the smoke gate became a TWO-leg composed gate — the driver gained the tier build, the §15 paq assert, and a second ISO composed WITH the append; smoke-r9.expect gained the cpu stages carried from cpu-bringup.expect's vocabulary (one vocabulary, two drivers): the emission-order §15 boot incl. the bootconf seed line, the host-side hostfwd observation (ss over the emitter's slirp argv — the D-04 pair), the ladder in order PAIR-OK → ETHER-OK → KEYFS-OK → AUTHSRV-LISTEN → FACTOTUM-OK → LISTEN-17010 → DONE, the post-cpurc prompt, and the post-passes (zero cpurc: FAIL, zero panic, ZERO latin1.8.font markers — DESK-02's backstop in the standing gate, bootconf + DONE present whole-capture). expect_before gained the cpurc: FAIL fast-fail arm. The existing chain leg's stages are byte-untouched inside the same runs. GREEN TWICE consecutively (three total green full runs; runs A/B captures distinct — independent per-bake domains).
- **Task 3 (f58ff7f81):** the README gained the full-system boot section (the --system flag table, the ONE-command recipe with what comes up in ladder order, the fail-loud rules, the frozen-aarch64 note), the full-chain smoke-leg section with its honest scope boundary (bring-up vs 19-08's interactive-session gate — one gate one concern), and the --system surface in the one-command block + the scripts table.

## Task Commits

1. **Task 1: boot-r9-x64 full-system mode — --system=cpu|terminal** - `7a2e32c3b` (feat)
2. **Task 2: the full-chain smoke leg, green twice + evidence** - `2f49d6eb3` (feat)
3. **Task 3: the ONE-command and full-chain README sections** - `f58ff7f81` (docs)

**Plan metadata:** (this SUMMARY + STATE/ROADMAP — the docs commit follows)

## Files Created/Modified

- `nix/boot-r9-x64.sh` — the --system surface: parse, image seam, append compose, tier build in cpu mode, system banner
- `nix/smoke-r9-x64.sh` — the cpu leg's pipeline: emitter capture, tier build, §15 assert, the cpu ISO, two-capture gate step
- `build/x86_64/smoke-r9.expect` — the cpu stages, hostfwd_stage procs, the cpurc:FAIL fast-fail, the desktop-absent post-pass
- `flake.nix` — the smoke app's emitter export + iproute2/procps (the deviation below)
- `build/x86_64/README.md` — the ONE-command + full-chain sections
- `evidence/19-09-smoke-{chain,cpu}-{a,b}.log` + `19-09-smoke-{a,b}-stages.txt` — the twice-green captures + stage transcripts

## Decisions Made

See key-decisions. Additionally: the second spawn reuses the same R9_RUN_TAG (the survivor sweep keys on the tag and now covers both spawns — the sibling-gate no-cross-kill rule unchanged); the cpu leg's capture is a separate file (R9_CPU_LOG, default build/x86_64/smoke-r9-cpu.log) so each leg's whole-file post-passes stay independent evidence; TMPD2 composes the cpu ISO so the two ISOs never collide.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] cpu mode runs the tier build (subset-build.sh) in step 1**
- **Found during:** Task 1 (the image-seam wiring)
- **Issue:** mkimage §15 names subset-build.sh as the producer of cmd/cpu, aux:listen, sed, and ipconfig — the boot app's step 1 ran only stage-build.sh, so off a clean checkout `--system=cpu` would die in the bake; the roadmap's ONE-command promise requires the full system to be ONE command
- **Fix:** cpu mode's step 1 runs stage-build.sh then subset-build.sh (the auth/drawterm gates' step-1 shape); terminal mode's step 1 is untouched
- **Files modified:** nix/boot-r9-x64.sh
- **Verification:** the live --system=cpu boot ran the full pipeline green (tier build line present in the log, then the ladder)
- **Committed in:** 7a2e32c3b

**2. [Rule 3 - Blocking] the smoke app needed the emitter export + iproute2/procps in flake.nix**
- **Found during:** Task 2 (wiring the cpu leg)
- **Issue:** the plan's files_modified list has no flake.nix, but the leg's qemu spawn consumes R9_NETDEV_ARGV (captured from R9_NETDEV_EMITTER — the D-04 hostfwd pair as pinned TOML data, never script text) and its hostfwd_stage runs `exec ss` — neither the env var nor ss existed in the smoke app's environment
- **Fix:** smokeR9X64Pkg exports R9_NETDEV_EMITTER and gains iproute2 + procps (the net gate's always-listed-dep rule, now covering the two-spawn sweep); the frozen x64 hasAttr gate list is byte-untouched (netdev-args has gated the whole block since 16-04 — no pin-advance choreography)
- **Files modified:** flake.nix
- **Verification:** the app set evals (21 apps) and the gate ran green three times through the app
- **Committed in:** 2f49d6eb3

---

**Total deviations:** 2 auto-fixed (1 missing critical, 1 blocking)
**Impact on plan:** Both were prerequisites for the plan's own verifies (the ONE-command promise; the leg's wiring). No scope creep — flake.nix is the app registration the plan itself says already exists ("no new apps"), extended in place.

## Issues Encountered

None. (The waived #66/CR-07 class never fired — the smoke's shape is not the 8-way concurrent bulk transfer; D-09's record-enrich arm unused.)

## Known Stubs

None - no stubs, placeholders, or unwired data paths were introduced.

## Threat Flags

None. The --system flag's trust boundary (user argv → mode selection) is covered by the plan's threat model: T-19-21 mitigated (single-accepted-form parse, usage errors naming the valid set, the default path pinned by the untouched-smoke run), T-19-22 mitigated (the diff-assert across the plan's commits shows zero aarch64/frozen-app files touched), T-19-SC honored (zero package installs).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 19-10 (the close-out/clean-checkout proof) rides the surfaces this plan landed: `nix run .#boot-r9-x64 -- --system=cpu` is the full-system entry and `nix run .#boot-r9-smoke-x64` is the standing full-chain gate — both green at the df4ad78 pin with the tree as committed here
- No kernel bytes were touched (r9 stays at df4ad78 == origin/main; flake.lock unchanged) — the FOD does not re-stale
- The x64 hasAttr gate list is byte-untouched; the smoke app's new emitter consumption rides the existing netdev-args gating

## Self-Check: PASSED

- Created/modified files exist: the 5 production files + 6 evidence files + the SUMMARY — all FOUND
- Task commits exist: 7a2e32c3b, 2f49d6eb3, f58ff7f81 all ancestors of HEAD (f58ff7f81)
- Evaluation scope: `gsd_run check evaluation-scope --plan 19-09 --commits-only` resolved with the 3 plan commits on this branch
- Plan verify legs re-run at close-out: MODE-WIRED, SYNTAX-OK (both scripts), DEFAULT-PATH-UNTOUCHED (Task 1's smoke exit 0), CHAIN-SMOKE-A-GREEN, CHAIN-SMOKE-GREEN-TWICE, VOCAB-CARRIED, DESKTOP-ABSENT-POSTPASS-PRESENT, README-ONECMD-OK — all pass
- Prohibition backstops: zero aarch64/frozen-app files in the plan diff (FROZEN-SURFACES-UNTOUCHED); the terminal default path byte-untouched (bake invocation + compose argv identical; the chain leg green in every run)

---
*Phase: 19-cpu-service-drawterm-rio-desktop*
*Completed: 2026-11-15*
