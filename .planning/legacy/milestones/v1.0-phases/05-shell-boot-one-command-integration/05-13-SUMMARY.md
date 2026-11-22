---
phase: 05-shell-boot-one-command-integration
plan: 13
subsystem: infra
tags: [r9, nix, flake, github-actions, ci, reproducibility, INT-02, INT-03, D-13, evidence, clean-checkout, qemu-virt, docs-tour]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-11)
    provides: inputs.r9 pinned to 9549cee (pushed to origin first — clean-clone resolvable) + the boot-r9 compose app + the native-linux subset-build leg this plan's CI/linux legs exercise
  - phase: 05-shell-boot-one-command-integration (05-12)
    provides: nix run .#boot-r9-smoke — the packaged expect smoke both clean-checkout legs and the CI workflow run verbatim
provides:
  - ".github/workflows/r9-shell.yml — the D-13 GH Actions linux leg: clean checkout → SHA-pinned nix install → nix run .#boot-r9-smoke on ubuntu-latest, 90-min budget, alongside the untouched harvey.yml"
  - "build/REPRODUCIBILITY.md's INT-03 section — policy/scope/derivation/override for the clean-checkout boot claim"
  - ".planning/phases/05-shell-boot-one-command-integration/05-EVIDENCE.md — the phase's canonical evidence bundle, r9 HEAD pinned (cbbc8e4, == live HEAD at evidence-commit time, the D-13 gate)"
  - "two clean-checkout proofs: 05-13-darwin-clean-clone.log + 05-13-linux-clean-clone.log (native aarch64-linux), both ALL STAGES PASS at dc8299575"
  - "../r9 docs tour: HowItWorks.md's 05-04 namespace-core + 05-10 flake-package/R9_HARVEY_DIR sections; README.md's virtshell/kernel-virt/R9_HARVEY_DIR rows"
affects: [verify-phase (the phase's evidence is consolidated and verifiable), milestone completion (INT-02/INT-03 close here), the first-push GH run (r9-shell.yml goes live)]

# Actuals (#2632) — pairs with the plan's estimate (100000 tokens, confidence: low).
actuals:
  tokens: 88000    # MEASURED chars/4: harvey diff 322779 chars (dominated by the two ~111KB evidence logs; authored ≈ 100KB) + r9 docs diff 12284 + this SUMMARY ~17000 = ~352000/4 ≈ 88000.  The low-confidence estimate was close this time — the evidence-log volume dominated exactly as flagged
  tasks: 3
  commits: 4       # MEASURED: harvey git rev-list --count a57917193..ffb7df8fa = 3 (the metadata docs commit rides on top); r9: 1 (cbbc8e4, the docs tour — committed FIRST per D-13)
plan_head_before: {harvey: "a5791719361f37d2678a93ab8b55843ee8771722", r9: "9549cee63d86a0c6d9b4d04ed8860b2efabcd3e6"}
plan_head_after: {harvey: "ffb7df8fa0ed4720db9bf08890459604a42ed389 (pre-metadata; the docs commit rides on top)", r9: "cbbc8e41719493e151540b224990ec9ef273d607"}

# Tech tracking
tech-stack:
  added: []        # zero new dependencies (T-05-SC held): the one new external reference is the SHA-pinned nix-installer-action (T-05-24), no packages installed anywhere
  patterns:
    - "CI leg shape for this repo: checkout → DeterminateSystems nix-installer-action PINNED TO A 40-CHAR COMMIT SHA (v23 = 3138316d…) → nix run .#<app>; permissions: contents: read; 90-min TCG budget; lands alongside the legacy workflow, never edits it"
    - "no-push CI validation: when pushing is prohibited, the workflow's encoded pipeline is executed natively on linux from a pristine clone (lima guest, sudo unshare, TCG) — the green run is the evidence; the hosted run URL is recorded as pending, never fabricated"
    - "clean-clone evidence discipline: local-path clone at the committed HEAD (the point is a pristine tree, not network provenance); the log self-describes clone SHA + host + command + exit; two-host legs are proven independent by bake-size + paqfs-walk-order diffs"

key-files:
  created:
    - .github/workflows/r9-shell.yml
    - .planning/phases/05-shell-boot-one-command-integration/05-EVIDENCE.md
    - .planning/phases/05-shell-boot-one-command-integration/05-13-darwin-clean-clone.log
    - .planning/phases/05-shell-boot-one-command-integration/05-13-linux-clean-clone.log
  modified:
    - build/REPRODUCIBILITY.md
    - ../r9/HowItWorks.md
    - ../r9/README.md

key-decisions:
  - "GH-run leg substituted per the orchestrator's explicit no-push rule: the workflow was authored + locally validated (YAML parse, smoke step, SHA pin, harvey.yml untouched), and the EXACT pipeline it encodes ran green natively on aarch64-linux from a pristine clone at dc8299575 — 05-EVIDENCE.md records the hosted run as PENDING FIRST PUSH (the gh run list query + actions URL named, nothing fabricated)"
  - "DeterminateSystems nix-installer-action pinned to commit 3138316df39ed29be04236d7ffc686fa525866aa (v23, lightweight tag → commit resolved via the GitHub API; T-05-24's supply-chain rule, gate-enforced by the plan's verify grep)"
  - "r9 docs committed FIRST (cbbc8e4), then the Harvey evidence commit recorded that exact HEAD — the D-13 ordering gate verified mechanically at commit time (recorded SHA == git -C ../r9 log -1 --format=%H)"
  - "the flake.lock r9 pin stays at 9549cee (NOT re-pinned to cbbc8e4): the docs commit is docs-only (verified diff: HowItWorks.md + README.md only — same kernel image), and re-pinning to an unpushed local commit would break the clean-checkout leg the pin exists to serve"

patterns-established:
  - "the two-repo evidence close-out: r9 docs commit → live-HEAD capture → Harvey EVIDENCE commit with the SHA gate re-run at commit time"

requirements-completed: [INT-02, INT-03]  # INT-02's shared-ID gate (#2388) clears: 05-12's SUMMARY exists; INT-03 is 05-13-only

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: ".github/workflows/r9-shell.yml authored per D-13: on [push, pull_request], one ubuntu-latest job, checkout → nix-installer-action@3138316d…(SHA-pinned) → nix run .#boot-r9-smoke, timeout-minutes 90, permissions contents: read, comment header naming D-13 + the SC#3 amendment; the legacy harvey.yml untouched"
    requirement: INT-02
    verification:
      - kind: unit
        ref: "python3 yaml.safe_load parse + 'boot-r9-smoke' step assertion (PASS); grep -qE 'nix-installer-action@[0-9a-f]{40}' (PASS); git diff --exit-code .github/workflows/harvey.yml (PASS — untouched)"
        status: pass
    human_judgment: false
  - id: D2
    description: "the linux clean-checkout pipeline proof (standing in for the hosted GH run, which is pending first push — harvey push prohibited this session): a pristine clone at dc8299575 ran nix run .#boot-r9-smoke natively on aarch64-linux (lima guest, sudo unshare namespace build, TCG qemu) to ALL STAGES PASS — the exact command sequence the workflow encodes"
    requirement: INT-02
    verification:
      - kind: e2e
        ref: "05-13-linux-clean-clone.log (committed): clone HEAD dc8299575, host Linux aarch64, SUBSET-BUILD-OK, baked bootfs.paq 1016045 bytes (magics asserted), 'smoke-r9: ALL STAGES PASS', exit 0; per-marker greps (banner/paqfs/stacksmoke/r9shell7/bind -c #e /env/r9delok/init×2) all present; FAUTH ×2 designed; zero panic/TIMEOUT; zero qemu orphans"
        status: pass
    human_judgment: false
  - id: D3
    description: "the INT-03 darwin leg: a pristine clone at dc8299575 on this darwin-aarch64 host ran the documented command sequence (nix run .#boot-r9-smoke — full default pipeline: lima userland build, bake, DTB patch, expect sweep) to ALL STAGES PASS, rc=0"
    requirement: INT-03
    verification:
      - kind: e2e
        ref: "the plan's literal verify: CLONE=$(mktemp -d)/harvey-clone && git clone --quiet . $CLONE && cd $CLONE && nix run .#boot-r9-smoke → 'clean-clone smoke rc=0'; evidence retained at 05-13-darwin-clean-clone.log (committed, 111668 bytes)"
        status: pass
    human_judgment: false
  - id: D4
    description: "build/REPRODUCIBILITY.md's INT-03 section: policy (process-level reproducibility + the Phase-1 KERNDATE policy's application to the aarch64 userland build), scope (pins named — r9 rev+narHash, nixpkgs rev, rust-toolchain.toml channel, qemu via nixpkgs; bit-identity explicitly NOT claimed), per-host derivation (darwin lima / linux native unshare / the CI leg), overrides (--override-input r9, R9_SKIP_BUILD, KERNDATE)"
    requirement: INT-03
    verification:
      - kind: unit
        ref: "grep chain PASS: INT-03, boot-r9-smoke, flake.lock, override-input, KERNDATE all present"
        status: pass
    human_judgment: false
  - id: D5
    description: "05-EVIDENCE.md consolidates the phase's canonical evidence: all five SCs mapped to quoted marker lines, the D-14 note-race record (root cause + fix SHA 83cbbbe + the 40/40 burial), the 05-03 measured 26-syscall set, the regression matrix, the known-deferred inventory; the recorded r9 SHA equals the live r9 HEAD at evidence-commit time (D-13 ordering gate, re-run at commit time)"
    requirement: INT-03
    verification:
      - kind: unit
        ref: "grep gates PASS: live SHA cbbc8e41… present; SC#1..SC#5 rows present; 40/40 present; the CI reference (gh run list --workflow r9-shell.yml / actions URL, marked pending-first-push) present"
        status: pass
    human_judgment: false
  - id: D6
    description: "r9 docs tour: HowItWorks.md gains the 05-04 namespace-core section (namec/pgrp/union walk/registry, rfork semantics, the measured set, eager data+bss, config_virtshell) and the 05-10 section (packages.kernel-virt + the R9_HARVEY_DIR-required tooling contract); README.md gains the virtshell profile row, the nix kernel-virt packages section, and the boot-test/ureg-check env contract note"
    verification:
      - kind: unit
        ref: "grep gates PASS: HowItWorks.md namespace|devmnt; README.md kernel-virt + R9_HARVEY_DIR; git ls-files --error-unmatch both (tracked); committed r9-side FIRST (cbbc8e4) per D-13"
        status: pass
    human_judgment: false
  - id: D7
    description: "a real GH-hosted run of r9-shell.yml going green on ubuntu-latest (x86_64-linux) — the plan's Task-1 acceptance in its literal form"
    requirement: INT-02
    verification: []
    human_judgment: true
    rationale: "pushing harvey to origin was prohibited for this execution (orchestrator rule), and gh is not installed on this host — no hosted run exists to observe. The workflow file is authored and locally validated (D1), and its exact encoded pipeline is proven green natively on aarch64-linux from a clean checkout (D2); the x86_64-linux first run happens at first push (05-11's IFD deferral: only a native x86_64-linux host can evaluate that leg). A human pushes and watches gh run list --workflow r9-shell.yml."

# Metrics
duration: 9 min
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 13: CI leg + clean-checkout proofs + the canonical evidence bundle Summary

**The phase's publication is complete: `.github/workflows/r9-shell.yml` lands the D-13 GH Actions linux leg (clean checkout → SHA-pinned nix install → `nix run .#boot-r9-smoke` on ubuntu-latest, alongside the untouched legacy harvey.yml); INT-03 is evidenced by TWO clean-checkout runs of the documented command — a pristine clone at `dc8299575` went green on darwin (lima build leg) AND natively on aarch64-linux (lima guest, `sudo unshare`, TCG — the exact pipeline the workflow encodes, run in place of the hosted run since pushing harvey was prohibited this session); `build/REPRODUCIBILITY.md` gains the INT-03 policy/scope/derivation/override section naming every pin; and `05-EVIDENCE.md` consolidates the phase's canonical bundle with the r9 SHA gate held mechanically (recorded `cbbc8e4` == live r9 HEAD at evidence-commit time — the r9 docs commit landed FIRST per D-13).**

## Performance

- **Duration:** 9 min (16:06:56Z → 16:15:41Z)
- **Started:** 2026-11-03T16:06:56Z
- **Completed:** 2026-11-03T16:15:41Z
- **Tasks:** 3
- **Files modified:** 7 (created: r9-shell.yml, 05-EVIDENCE.md, two clean-clone logs; modified: build/REPRODUCIBILITY.md, ../r9/HowItWorks.md, ../r9/README.md)

## Accomplishments

- **Task 1 — the CI leg (D-13):** `.github/workflows/r9-shell.yml` authored and locally validated (YAML parse + smoke-step assertion, the 40-char SHA pin grep, harvey.yml-untouched diff — all PASS). The nix-installer-action pin is v23 resolved to commit `3138316df39ed29be04236d7ffc686fa525866aa` via the GitHub API (T-05-24). With the harvey push prohibited, the linux-leg green evidence came from running the workflow's exact encoded pipeline natively: a pristine clone (carrying the workflow at `dc8299575`) rsynced into the lima aarch64-linux guest ran `nix run .#boot-r9-smoke` end-to-end — toolchain bootstrap, `SUBSET-BUILD-OK`, bake (1016045 bytes, paqfs magics asserted), TCG boot to **ALL STAGES PASS**, exit 0, zero qemu orphans. The serial capture is provably independent of the darwin evidence (different bake size, different paqfs root-walk order).
- **Task 2 — INT-03's darwin leg + the doc:** the plan's literal verify — `git clone` into a fresh temp dir → `nix run .#boot-r9-smoke` — ran green on this host (`clean-clone smoke rc=0`; evidence retained as `05-13-darwin-clean-clone.log`). `build/REPRODUCIBILITY.md` gained the INT-03 section in the file's existing Policy→Scope→Derivation→Override shape: process-level reproducibility (KERNDATE policy applied to the aarch64 userland build), the named pins (r9 `9549cee` + narHash, nixpkgs `31ecbaca…`, r9's `nightly-2026-01-30` via rust-toolchain.toml+rust-overlay, qemu via nixpkgs), the explicit NOT-claimed (bit-identical images — mkpaqfs embeds KERNDATE-derived metadata), the per-host command derivation, and the two non-default overrides.
- **Task 3 — the evidence bundle + r9 docs tour (D-13 ordering):** r9 committed FIRST (`cbbc8e4`): HowItWorks.md gained the two missing sections its header mandates — the 05-04 namespace core (namec/pgrp/union walk/registry, rfork share/copy/wipe, the measured syscall set, eager data+bss, config_virtshell) and 05-10's packages.kernel-virt + R9_HARVEY_DIR tooling contract — and README.md gained the virtshell profile row, the `nix build .#kernel-virt` packages section, and the boot-test/ureg-check env contract note. Then `05-EVIDENCE.md` consolidated the phase: SHA pins (live-gated), the SC#1–SC#5 mapping with quoted marker lines, the D-14 note-race record (root cause + `83cbbbe` + 40/40 burial), the 05-03 measured 26-syscall set, the regression matrix, and the known-deferred inventory — including the honest pending-first-push record for the hosted GH run.

## Task Commits

Each task was committed atomically (two-repo convention — r9 via `git -C` from the pinned harvey root):

1. **Task 1: .github/workflows/r9-shell.yml — the linux CI leg (D-13)** — harvey `dc8299575` (feat)
2. **Task 2: REPRODUCIBILITY.md INT-03 section + darwin clean-clone evidence** — harvey `cdc0ec775` (docs)
3. **Task 3 (r9 side, FIRST per D-13): HowItWorks namespace-core + flake-package tour; README rows** — r9 `cbbc8e4` (docs)
4. **Task 3 (harvey side): 05-EVIDENCE.md + the linux clean-clone log** — harvey `ffb7df8fa` (docs)

**Plan metadata:** (the docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS)

## Files Created/Modified

- `.github/workflows/r9-shell.yml` — NEW: the D-13 linux CI leg (40 lines)
- `build/REPRODUCIBILITY.md` — the INT-03 section (+86 lines)
- `.planning/phases/05-shell-boot-one-command-integration/05-EVIDENCE.md` — NEW: the canonical bundle
- `.planning/phases/05-shell-boot-one-command-integration/05-13-darwin-clean-clone.log` — NEW: the darwin leg's retained serial capture (111,668 bytes)
- `.planning/phases/05-shell-boot-one-command-integration/05-13-linux-clean-clone.log` — NEW: the linux leg's full pipeline log (3,231 lines)
- `../r9/HowItWorks.md` — +23 lines (the 05-04 + 05-10 sections)
- `../r9/README.md` — +9 lines (virtshell row, kernel-virt section, R9_HARVEY_DIR contract)

## Decisions Made

See key-decisions: the GH-run substitution (no-push rule), the v23 SHA pin, the D-13 ordering with the commit-time gate re-run, and the deliberate non-re-pin of flake.lock (docs-only delta; an unpushed re-pin would break clean checkouts).

## Deviations from Plan

### Auto-fixed / substituted Issues

**1. [Orchestrator override — verification venue] The GH-hosted run leg was replaced by a native linux clean-checkout run**
- **Found during:** Task 1 verification
- **Issue:** The plan's Task 1 action step 3 orders `git push` + `gh run watch` until green, and its acceptance criterion requires "a real GH run of it is green; the run URL/id is recorded." The orchestrator's execution context for this plan explicitly prohibits pushing harvey ("do not push harvey; follow the plan's stated verification approach — actionlint/local validation/lima linux leg"), and `gh` is not installed on this host.
- **Fix:** Authored + locally validated the workflow per spec (all non-push verify legs PASS), then executed the EXACT pipeline the workflow encodes — clean checkout → nix → `nix run .#boot-r9-smoke` — natively on aarch64-linux (the lima guest) from a pristine clone at `dc8299575` that carries the workflow. ALL STAGES PASS, exit 0. The hosted run is recorded in 05-EVIDENCE.md as PENDING FIRST PUSH with the query command/URL named — nothing fabricated. Coverage D7 keeps the hosted-run gap visible for human sign-off.
- **Files modified:** none beyond the plan's set (the substitution changed verification venue, not content)
- **Verification:** `05-13-linux-clean-clone.log` (committed) — clone SHA / host / pipeline stages / markers / exit all quoted and grepped
- **Committed in:** evidence lands in `ffb7df8fa`

**2. [Rule 3 - Blocking] Pre-existing case-collision checkout state surfaced by the clean clones**
- **Found during:** Task 1's clone (git warned: `rc/bin/{Kill,kill}`, troff font/devpost case pairs collide on case-insensitive APFS)
- **Issue:** The repo carries case-sensitive path pairs; on this host's case-insensitive filesystem only one variant per pair materializes, which also explains the pre-existing "modified" entries in the working tree's `git status` (present before this plan; NOT caused by it). The guest clone showed the missing variants as deleted.
- **Fix:** None needed — verified the colliding paths (troff fonts, postscript devpost tables, an rc convenience script) are outside the aarch64 subset build's compile set; both clean-clone legs ran green regardless. The working-tree state was left untouched (scope boundary); noted in 05-EVIDENCE.md's deferred inventory is NOT needed (pre-existing upstream tree shape, not a phase defect) — recorded here for visibility.
- **Files modified:** none
- **Verification:** both clone legs green end-to-end
- **Committed in:** n/a

---

**Total deviations:** 2 (1 orchestrator-directed venue substitution, 1 pre-existing-environment note)
**Impact on plan:** The venue substitution is exactly the fallback the orchestrator named; the pipeline-under-test is identical and the linux green run is real. The INT-02 linux evidence and INT-03 linux clean-checkout proof stand on it. No assertion was weakened; the hosted-run gap is documented, greppable, and routed to human judgment (coverage D7).

## Issues Encountered

- The linux leg's 40-second wall time warranted scrutiny (a cold pipeline "that fast" is a classic stale-evidence smell). Verified genuine: the kernel-virt store path's registrationTime falls inside the run window; the toolchain bootstrap, subset build, bake (distinct byte count), and the live interactive boot (echo/DEL round-trips, independent paqfs walk order) are all in the capture; the expect driver asserts markers over the live serial wire against a fresh log_file — unreplayable. vz-accelerated guest + same-ISA TCG is simply fast.
- One false alarm during hygiene checks: an unbracketed `pgrep -f qemu-system-aarch64` self-matched the checking shell's own argv (the phase's documented bracket-fix pitfall). Re-checked with the bracket-fixed pattern: zero orphans on host and guest.
- The 05-09 input-wedge flake did not reproduce in any run this plan (linux leg + darwin leg: 2/2 clean; 05-12+05-13 total 4/4).

## Authentication Gates

None triggered. (`gh` absent + harvey push prohibited — handled as the orchestrator-directed venue substitution above, not as an auth gate.)

## User Setup Required

None - no external service configuration required. **One human action remains by design:** push harvey to origin when ready — `.github/workflows/r9-shell.yml` then runs on ubuntu-latest, and its first green run's URL/id completes the SC#3 record in `05-EVIDENCE.md` (the pending marker is greppable).

## Known Stubs

None. Every deliverable is fully wired: the workflow encodes the exact proven pipeline; the EVIDENCE bundle's one pending item (the hosted run) is explicitly marked pending, not stubbed-as-done.

## Threat Flags

None beyond the plan's register. T-05-24 held (the nix-install action is pinned to a full 40-char commit SHA, gate-enforced by the plan's own grep; permissions are the default checkout scope, made explicit as `contents: read`). T-05-25 held (the D-13 ordering gate was re-run mechanically at evidence-commit time; every SC row quotes its log lines; the one unmet row is marked pending, never claimed). T-05-SC held (zero package installs this plan).

## Next Phase Readiness

- **Phase 5 is complete pending verification** — all 13 plans have SUMMARYs; `/gsd-verify-phase 05` is the next step (INT-02/INT-03 flip Complete with this plan).
- The hosted CI run goes live on the next harvey push; r9 needs no push for this plan's correctness (the flake.lock pin at `9549cee` already resolves from clean clones; `cbbc8e4` is docs-on-top — push it with the next r9 code push per convention).
- Milestone backlog pointer: 05-EVIDENCE.md's known-deferred inventory.

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: `.github/workflows/r9-shell.yml`, `build/REPRODUCIBILITY.md`, `05-EVIDENCE.md`, `05-13-darwin-clean-clone.log`, `05-13-linux-clean-clone.log`, this SUMMARY — all FOUND on disk; `../r9/HowItWorks.md` + `../r9/README.md` modifications committed r9-side.
- Commits: harvey `dc8299575` (Task 1), `cdc0ec775` (Task 2), `ffb7df8fa` (Task 3 evidence) — all FOUND in `git log --oneline --all`; r9 `cbbc8e4` FOUND at r9 main HEAD. Ledger-measured: `git rev-list --count a57917193..ffb7df8fa` = 3 harvey (+1 r9 docs).
- Plan-level <verification> re-run at final state: (1) CI legs — workflow parses with the smoke step (PASS), SHA pin grep (PASS), harvey.yml untouched (PASS); the hosted-run leg substituted per the orchestrator's no-push rule with the native aarch64-linux clean-clone run green (deviation 1, coverage D7). (2) INT-03 — the plan's literal clean-clone verify ran green on darwin (rc=0); REPRODUCIBILITY.md greps all PASS. (3) Evidence — the recorded r9 SHA `cbbc8e41…` == live r9 HEAD (re-checked at the evidence commit), SC#1–SC#5 rows + 40/40 + the CI reference all present; r9 docs greps PASS.
- Acceptance criteria re-run at final state: Task 1 (workflow alongside untouched harvey.yml, pinned-SHA install, ubuntu-latest, 90-min budget, full pipeline — PASS; "real GH run green + URL recorded" — SUBSTITUTED per the no-push rule: the encoded pipeline ran green natively on linux from a clean clone; the hosted run is recorded pending-first-push). Task 2 (pristine clone green on darwin — PASS; REPRODUCIBILITY.md section complete — PASS). Task 3 (all five SCs mapped with SHA pins; recorded SHA == live HEAD — PASS; docs tour + CI reference + 40/40 record in the bundle — PASS).
