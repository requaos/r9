---
phase: 05-shell-boot-one-command-integration
plan: 12
subsystem: testing
tags: [r9, nix, flake, expect, smoke, boot-r9-smoke, INT-02, D-09, D-12, D-13, D-16, qemu-virt, TCG]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-11)
    provides: the boot-r9 compose app + the R9_KERNEL_PKG store-path contract — the smoke runs the SAME pipeline pieces (build → bake → fdtput DTB patch) and the same flake-input kernel
  - phase: 05-shell-boot-one-command-integration (05-09)
    provides: drive-r9.expect — the dev-harness sweep driver whose assertion set the packaged smoke grows from (same markers, CI-shaped hygiene and exit discipline)
provides:
  - "nix run .#boot-r9-smoke — INT-02's expect-style smoke as a flake app (D-12): full default pipeline (lima/native userland build → mkimage bake → DTB patch → expect-driven qemu) asserting the whole D-09 sweep, exit 0/1 for CI"
  - "build/aarch64/smoke-r9.expect — the packaged sweep driver (grown from drive-r9.expect; spawns qemu directly over -nographic stdio, per-stage 600s named timeouts, presence-only CRLF-tolerant markers, fresh log_file per run, group-kill teardown)"
  - "nix/smoke-r9.sh — the app script: R9_KERNEL_PKG store-path contract (T-05-21), rm -f capture before launch, EXIT-trap bracket-fixed pkill fallback keyed on this run's kernel path"
  - ".planning/phases/05-shell-boot-one-command-integration/05-12-smoke.log — retained darwin pass evidence (run 2 of the two-consecutive-pass gate; 111689 bytes)"
affects: [05-13 (the CI linux leg runs this exact app — the darwin leg is green here), INT-02, ROADMAP SC#3-as-amended (D-13), verify-work]

# Actuals (#2632) — pairs with the plan's estimate (80000 tokens, confidence: low).
actuals:
  tokens: 36500    # MEASURED chars/4 over the realized diff: 133303 chars of diff (dominated by the 111689-byte retained evidence log; authored code = 209+187 lines ≈ 14.5K chars) + this SUMMARY ~12.5K chars → ~146000/4 ≈ 36500.  The estimate's low confidence was right — the sweep driver was a proven grow-from (05-09), the pipeline a mirror (05-11), and the host was warm (lima guest + nix store replay made each full-pipeline gate run ~2-3 min)
  tasks: 2
  commits: 3       # MEASURED: git rev-list --count c7f8eff23..e97bc2245 = 3 (harvey; the metadata docs commit rides on top).  r9: 0 — nothing r9-side this plan (D-12: r9's boot-test profiles stay unchanged r9-side)
plan_head_before: {harvey: "c7f8eff23dea8b676f85fd0d50f3444a73644062"}
plan_head_after: {harvey: "e97bc224501dbc5b5826c181116ee4d09357243b (pre-metadata; the docs commit rides on top)"}

# Tech tracking
tech-stack:
  added: []        # zero new dependencies (T-05-SC held): expect was already in Harvey's devshell/flake pins; the smoke drives existing in-repo machinery
  patterns:
    - "packaged-smoke contract: the expect driver spawns qemu DIRECTLY (the build/build.bash heredoc precedent — spawn/expect/send over -nographic stdio) with argv pieces passed as env from the app script; expect's spawn makes qemu a session/group leader on BOTH hosts, so one group-kill teardown works everywhere and the shell-side pkill is a pure fallback"
    - "stage-named failure localization: every expect stage reports 'smoke-r9: TIMEOUT at stage: <name>' on stderr — a failing boot names its stage (banner/paqfs/stacksmoke/prompt/echo/ls/ps/cat/DEL/restart), never a bare hang"
    - "fresh-capture hygiene that a pre-seeded stale log cannot pass (T-05-22): rm -f before launch in BOTH the shell caller and the driver, log_file opened fresh, markers are the only proof (the supervisor's exit status is never trusted), two-consecutive-pass gate (run 2 would inherit run 1's poison if any existed)"

key-files:
  created:
    - build/aarch64/smoke-r9.expect
    - nix/smoke-r9.sh
    - .planning/phases/05-shell-boot-one-command-integration/05-12-smoke.log
  modified:
    - flake.nix
    - .gitignore

key-decisions:
  - "the smoke spawns qemu directly from expect with -nographic stdio (the A6 pick) — the build.bash/drive-r9-proven shape; no unix-socket serial was needed: no qemu monitor noise interleaves on the wire in practice (the compose-gate and both smoke runs show a clean sweep)"
  - "the retained evidence is run TWO's capture — the hygiene-proving second pass: run 1 green alone could theoretically ride a stale log or an orphan assist; run 2 green after run 1's rm -f proves the fresh-capture rule end-to-end (both runs' full logs additionally inspected — byte-shaped-identical, genuinely independent: distinct mktemp DTB dirs and temp object names in the diff)"
  - "shellcheck SC2086 in the EXIT trap (exit \"$status\") — same 0.11-info-fails-the-app-build class as 05-11's SC2329; caught by the first smoke run's app build before any boot, fixed, gate re-run to green"

patterns-established:
  - "flake smoke app shape: mkScriptApp over nix/<name>.sh with the R9_KERNEL_PKG header + runtimeInputs (expect added), the expect driver as a committed file under build/aarch64/, and the retained-evidence copy into the phase dir — 05-13's CI workflow consumes exactly this app"

requirements-completed: [INT-02]  # shared-ID gate (#2388): INT-02 is ALSO declared by 05-13 (the CI linux leg — no SUMMARY yet), so requirements.ready-ids blocks the Complete flip until 05-13 lands; the darwin leg this plan delivers is the smoke itself + its green proof

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "build/aarch64/smoke-r9.expect — the packaged sweep driver with every 05-09 assertion (banner → paqfs... → stacksmoke → %  prompt; echo r9shell7; ls /bin subset rc/ls/cat/echo/ps; ps ≥3 live-process lines incl rc; cat /lib/namespace's 'bind -c #e /env'; DEL safety via r9delok; exit → init's for(;;) restart to a fresh %), per-stage 600s named timeouts, CRLF-tolerant presence-only markers (D-13), fresh log_file per run, group-kill teardown on every exit path, exit 0 only on all-pass"
    requirement: INT-02
    verification:
      - kind: e2e
        ref: "nix run .#boot-r9-smoke ×2 (the plan's literal gate) — both runs ALL STAGES PASS; driver parse check: /usr/bin/expect smoke-r9.expect with no env exits 2 naming the missing contract var; greps: log_file + set timeout present"
        status: pass
    human_judgment: false
  - id: D2
    description: "nix/smoke-r9.sh + the boot-r9-smoke flake app — same R9_KERNEL_PKG store-path contract and pipeline pieces as boot-r9 (T-05-21; zero sibling-checkout paths in code lines), rm -f capture before launch, bracket-fixed pkill fallback, boot-r9's own behavior untouched (bootR9Pkg block byte-identical — additive-only diff)"
    requirement: INT-02
    verification:
      - kind: unit
        ref: "sh -n nix/smoke-r9.sh; grep smoke-r9.expect in the script; 'grep -vE '^\\s*#' | grep -c ../r9' == 0; grep -E 'pkill -f.*[q]emu' matches; nix flake show carries boot-r9-smoke; git diff of flake.nix shows additive-only"
        status: pass
    human_judgment: false
  - id: D3
    description: "TWO CONSECUTIVE green nix run .#boot-r9-smoke runs on darwin (the plan's literal verify: nix run .#boot-r9-smoke && nix run .#boot-r9-smoke → GATE_EXIT=0), each the FULL default pipeline (lima userland build → mkimage bake → fdtput DTB patch → expect sweep); the retained evidence (05-12-smoke.log, run 2's capture) carries every headline marker (banner, stacksmoke depth 64, r9shell7, bind -c #e /env, % , r9delok, init: starting /bin/rc ×2); zero qemu orphans after both runs"
    requirement: INT-02
    verification:
      - kind: e2e
        ref: "the &&-chained gate command exit 0; per-marker grep -qF against the retained log (all PASS, unambiguous); pgrep -f qemu-system-aarch64 empty post-gate"
        status: pass
    human_judgment: false
  - id: D4
    description: "the phase-gate human review of the retained log against the CONTEXT.md acceptance story (boot prints through connectembed and the mount, init starting /bin/rc, stacksmoke, the literal %  prompt, the sweep answering, DEL, restart) — the carried proof standard's human checkpoint"
    requirement: INT-02
    verification: []
    human_judgment: true
    rationale: "the plan's Task 2 carries an explicit <human-check> reserved to the end-of-phase UAT harvest (workflow.human_verify_mode=end-of-phase) — automation asserted every marker, but the acceptance-story read-through (and the optional hands-on nix run .#boot-r9) is the harvest's call by design"

# Metrics
duration: 6 min
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 12: The expect-style smoke as a flake app — INT-02's packaged proof, green twice on darwin Summary

**`nix run .#boot-r9-smoke` is INT-02's expect-style smoke (D-12): a new flake app drives the SAME composed pipeline as `boot-r9` (lima/native userland build → mkpaqfs bake → fdtput DTB patch → the flake-input kernel) but launches qemu under `build/aarch64/smoke-r9.expect`, which asserts the full D-09 sweep — banner → `paqfs...` → `stacksmoke: depth 64 ok` → the `% ` prompt → `echo r9shell7` → `ls /bin` subset → `ps` live-process lines incl rc → `cat /lib/namespace`'s `bind -c #e /env` → DEL-safety via `r9delok` → `exit` → init's `for(;;)` restart to a fresh prompt — with per-stage 600s named timeouts, presence-only CRLF-tolerant markers, a fresh capture per run, and a provable qemu kill on every exit path. The gate ran the plan's literal verify — `nix run .#boot-r9-smoke && nix run .#boot-r9-smoke` — TWO consecutive full-pipeline greens on this darwin host, run 2's capture retained as the phase evidence, zero qemu orphans.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-11-03T15:55:34Z
- **Completed:** 2026-11-03T16:01:38Z
- **Tasks:** 2
- **Files modified:** 5 (created: smoke-r9.expect, nix/smoke-r9.sh, the retained log; modified: flake.nix, .gitignore)

## Accomplishments

- **Task 1 — the packaged smoke:** `build/aarch64/smoke-r9.expect` grown from 05-09's drive-r9.expect (every assertion kept; spawns qemu directly over `-nographic` stdio — the build.bash-proven shape), `nix/smoke-r9.sh` mirroring boot-r9.sh's pipeline pieces and store-path contract, and `apps.boot-r9-smoke` wired in flake.nix (mkScriptApp `harvey-boot-r9-smoke`, same R9_KERNEL_PKG header, runtimeInputs + expect; `boot-r9` untouched — the diff is additive-only). Structural verifies: `sh -n` clean, expect-ref/log_file/timeout greps pass, zero `../r9` in code lines, bracket-fixed pkill present, `nix flake show` carries the app, driver parse-checked (exit 2 naming the env contract).
- **Task 2 — the INT-02 GATE PASS:** two consecutive `nix run .#boot-r9-smoke` runs on darwin, both ALL STAGES PASS end-to-end (stage table below). Each run was the FULL default pipeline — the lima userland build and the bake ran per run (no skip hatch), the kernel came from the flake-input store path (`ylgcqwi…-r9-kernel-virt-9549cee/kernel-virt.elf`, self-described in the argv line). Run 2's capture retained at `.planning/phases/05-shell-boot-one-command-integration/05-12-smoke.log` (111,689 bytes): every headline marker verified present with unambiguous per-marker greps; `init: starting /bin/rc` appears twice (boot + the exit-triggered restart — D-01's loop); zero qemu orphans afterward.
- **No assertion loosened; no flake encountered.** The 05-09 input-wedge flake (deferred-items top debug candidate) did NOT fire in any of the three runs this session (shakeout + gate ×2) — 3/3 clean. Had it wedged, the policy held ready: the failure names its stage, the wedge's signature (TIMEOUT at a prompt-read stage) would have been debugged against the boot, never weakened.

### Stage-timeout table (per-stage, TCG-sized — Pitfall 11)

| Stage | Assertion | Timeout |
|-------|-----------|---------|
| banner | `r9 from the Internet` | 600s |
| paqfs | `paqfs...` | 600s |
| stacksmoke | `stacksmoke: depth 64 ok` | 600s |
| prompt | `% ` | 600s |
| echo-response / echo-prompt | `r9shell7` / `% ` | 600s |
| ls-prompt + subset | `% `, then rc/ls/cat/echo/ps word-anchored in the capture | 600s |
| ps-prompt + shape | `% `, then ≥3 `glenda <pid>…` lines, one ending ` rc` | 600s |
| cat-prompt + known line | `% `, then `bind -c #e /env` in the capture | 600s |
| del-reprompt / del-answer / del-prompt | `% ` / `r9delok` / `% ` | 600s |
| restart-print / restart-prompt | `init: starting /bin/rc` / `% ` | 600s |

Every failure path prints `smoke-r9: TIMEOUT at stage: <name>` (or `qemu exited (EOF) at stage: <name>`) on stderr and exits 1 — never a bare hang.

## Task Commits

Each task was committed atomically (harvey-side only this plan — r9 untouched per D-12):

1. **Task 1: smoke-r9.expect + nix/smoke-r9.sh + the boot-r9-smoke app** — `a8cffd157` (feat; smoke-r9.expect, nix/smoke-r9.sh, flake.nix, .gitignore)
2. **Rule-1 fix: shellcheck SC2086 in the smoke EXIT trap** — `163682f60` (fix; nix/smoke-r9.sh)
3. **Task 2 (INT-02 GATE): two consecutive green runs + retained evidence** — `e97bc2245` (test; 05-12-smoke.log)

**Plan metadata:** (the docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS)

## Files Created/Modified

- `build/aarch64/smoke-r9.expect` — NEW: the packaged sweep driver (209 lines; grown from drive-r9.expect)
- `nix/smoke-r9.sh` — NEW: the app script (187 lines; pipeline + hygiene + driver launch)
- `flake.nix` — `smokeR9Pkg` + `boot-r9-smoke` app wiring (additive; bootR9Pkg byte-identical)
- `.gitignore` — `/build/aarch64/smoke-r9.log` (generated transient capture; the compose-gate.log precedent)
- `.planning/phases/05-shell-boot-one-command-integration/05-12-smoke.log` — the retained darwin evidence (run 2's capture, 100755 like the phase's other expect captures)

## Decisions Made

- **Direct qemu spawn over stdio (A6 pick)** — see key-decisions.
- **Run-two capture as the retained evidence** — see key-decisions.
- **R9_SKIP_BUILD=1 hatch carried into the smoke** — documented non-default, iteration-only (mirrors boot-r9.sh); the gate ran the default path both times. Consistency with the compose app beats a special case.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] shellcheck SC2086 in the EXIT trap failed the app build**
- **Found during:** Task 2's first smoke invocation (the app's writeShellApplication checkPhase — before any boot)
- **Issue:** `exit $status` unquoted in `cleanup()` — shellcheck 0.11 exits nonzero on INFO diagnostics (05-11's SC2329 class), failing the derivation
- **Fix:** `exit "$status"`
- **Files modified:** nix/smoke-r9.sh
- **Verification:** the app built; the shakeout run went ALL STAGES PASS; the two gate runs green
- **Committed in:** `163682f60`

**2. [Rule 3 - Blocking] .gitignore entry for the transient capture**
- **Found during:** Task 1 authoring
- **Issue:** the smoke produces `build/aarch64/smoke-r9.log` on every run — generated runtime output the plan's files_modified didn't list; the untracked-file protocol (and the 05-11 compose-gate.log precedent) requires it gitignored
- **Fix:** added `/build/aarch64/smoke-r9.log` to .gitignore
- **Files modified:** .gitignore
- **Verification:** `git status --short` shows no smoke artifacts after three runs
- **Committed in:** `a8cffd157` (part of the Task 1 commit)

**3. [Rule 2 - Missing Critical] the plan's evidence-grep chain could mask a missing marker**
- **Found during:** Task 2 verify
- **Issue:** the second automated leg's `A && B && … && pgrep && { exit 1; } || true` shape short-circuits to `|| true` on ANY grep miss — a missing marker and a full pass both exit 0 (the chain as written only fails loud on the orphan case, where `exit 1` precedes the `|| true`)
- **Fix:** ran the plan's chain as written (exit 0) AND re-ran every marker as an unambiguous per-marker `grep -qF` loop with explicit PASS/FAIL lines (all PASS) — assertions unchanged, only the reporting hardened
- **Files modified:** none (verify-venue only)
- **Verification:** per-marker output recorded above; the retained log carries every headline marker
- **Committed in:** n/a

---

**Total deviations:** 3 auto-fixed (1 Rule 1, 1 Rule 2, 1 Rule 3)
**Impact on plan:** All inside the plan's intent — one gate-caught tooling fix in my own new script (exactly what the app build's checkPhase exists to catch), one generated-artifact hygiene entry, one verify-hardening with the assertions preserved. No scope creep; no assertion weakened anywhere.

## Issues Encountered

- The first gate-attempt wrapper hit a `mktemp -t` dialect error (the orchestration shell picked up a GNU-flavored mktemp: "too few X's") — an artifact of MY log-capture wrapper, not the smoke; re-ran with explicit paths under the session temp dir. The smoke itself was unaffected.
- The input-wedge flake (deferred-items, top debug candidate for this plan): did NOT reproduce — 3/3 clean runs (one shakeout + two gate). The retry/assertion policy stayed armed but unused.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. The smoke is fully wired: flake app → compose pipeline → expect driver → marker assertions → exit 0/1. (R9_SKIP_BUILD=1 is a documented, non-default iteration hatch — not a stub.)

## Threat Flags

None beyond the plan's register. T-05-22 held (fresh capture per run in both the caller and the driver; two-consecutive-pass gate; markers are the only proof; the post-run orphan check is part of the verify). T-05-23 held (the driver group-kills its spawned qemu on every exit path; the shell's pkill fallback is bracket-fixed and keyed on this run's kernel path). T-05-SC held (zero package installs).

## Next Phase Readiness

- **05-13 (CI + clean checkouts):** inherits `nix run .#boot-r9-smoke` verbatim — the linux leg runs this exact app on ubuntu-latest (native-unshare build leg, TCG timeouts already sized at 600s/stage). INT-02's Complete flip is gated on 05-13's SUMMARY (shared-ID #2388). The input-wedge flake watch carries forward: 0/3 here, but the CI leg is its first high-volume venue.
- r9 HEAD: `9549cee` (unchanged this plan — D-12). Harvey HEAD: `e97bc2245` + the metadata commit.

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: `build/aarch64/smoke-r9.expect`, `nix/smoke-r9.sh`, `.planning/phases/05-shell-boot-one-command-integration/05-12-smoke.log` — all FOUND on disk; `flake.nix`/`.gitignore` modifications committed.
- Commits: `a8cffd157` (Task 1), `163682f60` (Rule-1 fix), `e97bc2245` (Task 2 gate + evidence) — all FOUND in `git log --oneline --all`. Ledger-measured count: `git rev-list --count c7f8eff23..e97bc2245` = 3.
- Plan-level <verification> re-run at final state: (1) structural — sh -n clean, expect-ref/log_file/timeout greps PASS, zero `../r9` in code lines, bracket-fixed pkill present, `nix flake show` carries boot-r9-smoke — PASS; (2) gate — `nix run .#boot-r9-smoke && nix run .#boot-r9-smoke` exit 0, both runs ALL STAGES PASS — PASS; (3) evidence — retained log non-empty with every headline marker (per-marker grep -qF, unambiguous) and zero qemu orphans — PASS; (4) human — deferred to the end-of-phase UAT harvest per workflow.human_verify_mode=end-of-phase (coverage D4 keeps it visible).
- Acceptance criteria re-run at final state: Task 1 (app exists with full sweep set, per-stage named timeouts, fresh-capture hygiene, bracket-fixed kill; no sibling-checkout paths; boot-r9 unregressed — additive-only flake diff) — PASS. Task 2 (two consecutive green runs with evidence retained and no leaks; complete acceptance story in the log; every failure mode names its stage; no assertion loosened) — PASS.
