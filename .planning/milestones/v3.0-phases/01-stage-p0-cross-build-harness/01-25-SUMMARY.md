---
phase: 01-stage-p0-cross-build-harness
plan: 25
subsystem: cross-build-harness
tags: [github-actions, ci, actionlint, nix-flake, nix-develop, harvey-tree, tcg, d-10, d-13, harn-07, harn-08]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-13/01-14's harvey-tree-arm64 and harvey-tree-amd64 (the flake supply the devshell exports); 01-21's boot-r9-smoke and boot-r9-tier1; 01-23's boot-r9-smoke-x64 app family; 01-30's boot-r9-net-smoke and boot-r9-cpu; 01-18's quick gate (cargo xtask gate)"
provides:
  - ".github/workflows/xtask.yml boot-test-x86-64: nix install (sha-pinned, flakes enabled), both read-only Harvey trees built from the flake, and the three q35 profiles run through `nix develop -c cargo xtask boot-test`; no Harvey checkout, no R9_HARVEY_DIR (HARN-07)"
  - ".github/workflows/gates.yml: one continue-on-error job per relocated leg (boot-r9-smoke, boot-r9-net-smoke, boot-r9-cpu, boot-r9-tier1 under explicit TCG, boot-r9-net-smoke-x64, boot-r9-auth-x64 under R9_ACCEL=tcg) plus quick-gate-tcg (D-10, D-13, HARN-08)"
  - "Every action in both workflows pinned by full 40-char commit sha; actionlint clean on both"
affects: [01-24, 01-27, 01-31, HARN-07, HARN-08, D-10, D-13]

actuals:
  tokens: 3359
  tasks: 2
  commits: 2

plan_head_before: de0c2024a9786b34d7c40c2bb103f5ae66db320c
plan_head_after: 85701ad7b992630b9f8eff0f443e40f2baa69fed

tech-stack:
  added: []
  patterns:
    - "CI runs the harness through `nix develop -c`: the devshell's shellHook exports are the supply, so a workflow never checks another repository out and never sets R9_HARVEY_DIR"
    - "Relocated best-effort legs are job-level `continue-on-error: true` with Harvey's timeout, sha-pinned actions and a header comment naming local KVM as the only witness (D-13)"

key-files:
  created:
    - .github/workflows/gates.yml
  modified:
    - .github/workflows/xtask.yml

key-decisions:
  - "All 11 legacy `actions/checkout@v3` references in xtask.yml are pinned to v4's API-resolved sha (11d5960a…), not just the boot-test job's. actionlint reds every `@v3` as an unsupported runner, so the task's own `<verify>` (clean actionlint, no error lines) could not pass otherwise; the plan's pinned-actions truth (T-01-80) and the verify both outrank 'leave other jobs unchanged' for these lines. No job's commands, runner or timeout changed."
  - "The apt boot-chain step is gone: the Linux devshell carries grub2, xorriso and qemu, and xtask's grub-mkrescue path is BIOS-only (no EFI image, so no mtools — xtask/src/main.rs:4452)."
  - "boot-r9-tier1 runs under explicit `R9_ACCEL=tcg`, and the two x64 legs keep Harvey's `env R9_ACCEL=tcg` shape. The other aarch64 apps are invoked exactly as Harvey invoked them (their scripts select TCG themselves on a KVM-less host); no new default is introduced."
  - "quick-gate-tcg gets a 120-minute timeout: there is no Harvey-hosted equivalent to match, and it runs the drawterm driver twice (rio, then control-noG) with the driver's 600 s per-stage budgets at TCG scale."
  - "No artifact-upload steps were carried over. The plan's job contract is the `nix run` invocation for each leg; the retained captures stay in the job log and under target/gates/, and the best-effort outcome is the job status."
  - "gates.yml adds `permissions: contents: read` (Harvey's posture): the legs build and boot locally and need no write scope (T-01-80's no-secrets mitigation)."

patterns-established:
  - "A relocated Harvey-hosted leg becomes one job in gates.yml: sha-pinned checkout + nix install, the single `nix run .#<app>` (or `env R9_ACCEL=tcg nix run`), job-level continue-on-error, Harvey's timeout"
  - "boot-test CI shape: install nix with flakes enabled → `nix build --no-link .#harvey-tree-amd64 .#harvey-tree-arm64` → `nix develop -c cargo xtask …`, which is the same supply path the devshell gives a developer"

requirements-completed: [HARN-07, HARN-08]

coverage:
  - id: D1
    description: "xtask.yml's boot-test-x86-64 job runs on the flake supply: nix installed (sha-pinned, flakes enabled), both read-only Harvey trees built from the pinned flake input, the three q35 profiles via `nix develop -c cargo xtask boot-test`, with no Harvey checkout and no R9_HARVEY_DIR left anywhere in the file"
    requirement: HARN-07
    verification:
      - kind: other
        ref: "nix run nixpkgs#actionlint -- .github/workflows/xtask.yml → rc 0, no output"
        status: pass
      - kind: other
        ref: "grep -cE 'R9_HARVEY_DIR|repository: requaos/harvey' .github/workflows/xtask.yml → 0"
        status: pass
      - kind: other
        ref: "grep -E 'uses: ' .github/workflows/xtask.yml | grep -vcE '@[0-9a-f]{40}' → 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "gates.yml holds one job per relocated leg (six) plus quick-gate-tcg, every job continue-on-error with Harvey's timeout, every action sha-pinned, and a header comment recording that the required witness is local KVM, never this workflow"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix run nixpkgs#actionlint -- .github/workflows/gates.yml → rc 0, no output"
        status: pass
      - kind: other
        ref: "grep -cE 'nix run \\.#boot-r9-(smoke|net-smoke|cpu|tier1|net-smoke-x64|auth-x64)\\b' .github/workflows/gates.yml → 6"
        status: pass
      - kind: other
        ref: "grep -c 'cargo xtask gate --arch x86-64 --accel tcg' .github/workflows/gates.yml → 1; grep -c 'continue-on-error: true' → 7"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every third-party action in both workflows is pinned by full commit sha (T-01-80)"
    verification:
      - kind: other
        ref: "grep -E 'uses: ' on both workflows | grep -vcE '@[0-9a-f]{40}' → 0 for each"
        status: pass
    human_judgment: false
  - id: D4
    description: "The two workflows run on GitHub after the owner pushes, and their results become the best-effort second witness"
    verification: []
    human_judgment: true
    rationale: "Hosted execution cannot happen from this worktree (no push, D-13). The plan marks this truth as a backstop and records the first results at stop E; also, boot-r9-net-smoke-x64 (plan 01-24, this wave) and boot-r9-auth-x64 (plan 01-31, next wave) do not exist at this base — the first push carrying this workflow is stop C's (plan 01-27), after 01-31, so no hosted run sees an app missing."

duration: 1 min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 25: CI on the flake supply, and the relocated gate legs Summary

**r9's boot-test CI job now installs nix, builds both read-only Harvey trees from the flake and runs the three q35 profiles inside `nix develop` — with no Harvey checkout and no `R9_HARVEY_DIR` — and a new `gates.yml` gives each Harvey-hosted r9 leg an r9 home plus the quick gate under explicit TCG, all best-effort and sha-pinned.**

## Performance

- **Duration:** 1 min (2026-11-23T14:19:49Z → 2026-11-23T14:20:24Z)
- **Tasks:** 2 of 2
- **Files modified:** 2 (1 created, 1 modified; 174 insertions, 35 deletions)
- **Plan estimate:** 40,000 tokens; **actual:** 3,359 tokens (chars/4 over 13,436 chars of realized diff). The wall clock went to reading Harvey's two workflows and the devshell's supply contract; the diff itself is small.

## Accomplishments

- **The boot-test job runs on the same supply a developer uses (HARN-07).** `boot-test-x86-64` installs nix with flakes enabled (`DeterminateSystems/nix-installer-action@3138316d…`, the pin Harvey's workflows already use), builds `.#harvey-tree-amd64` and `.#harvey-tree-arm64` from the pinned flake input, then runs the three q35 profiles as `nix develop -c cargo xtask boot-test --arch x86-64 --config <q35|q35lowmcfg|q35net> --accel tcg --cpu qemu64 --verbose`. The Harvey checkout step, its `R9_HARVEY_DIR` job env entry and the comment that named either are gone (`grep -c` = 0). `timeout-minutes: 120` is kept.
- **The tree derivation's venue is A16-safe.** The flake builds the trees through the proot venue, so the AppArmor-restricted hosted runner is fine; the comment records that and why the apt boot-chain and mtools steps are unnecessary (the Linux devshell carries grub2/xorriso/qemu; xtask's ISO is BIOS-only).
- **Every Harvey-hosted r9 leg has an r9 job (D-10, HARN-08).** `gates.yml` (on `push` and `workflow_dispatch`, `permissions: contents: read`) holds `boot-r9-smoke`, `boot-r9-net-smoke`, `boot-r9-cpu`, `boot-r9-tier1`, `boot-r9-net-smoke-x64` and `boot-r9-auth-x64`, each a sha-pinned checkout + nix install plus the single `nix run .#<app>` invocation Harvey used (the two x64 legs and tier-1 keep explicit TCG: `env R9_ACCEL=tcg`).
- **The quick gate is the best-effort second witness (D-13).** `quick-gate-tcg` runs `nix develop -c cargo xtask gate --arch x86-64 --accel tcg`. A header comment states the required witness is a local x86_64 KVM run, never this workflow, and that a hosted TCG result is never recorded as the D-13 KVM witness (HARN-01).
- **Every job is best effort and every action is pinned.** Seven job-level `continue-on-error: true` lines; the relocated legs keep Harvey's 90-minute timeouts (the quick gate gets 120 for its two driver legs); `grep -E 'uses: ' | grep -vcE '@[0-9a-f]{40}'` prints 0 for both files (T-01-80).
- **actionlint is clean on both workflows** (rc 0, no error lines), and Harvey is untouched (HEAD `25d97dc4d…` at the same dirty state it had before this plan; no write of any kind).

## Task Commits

Each task was committed atomically:

1. **Task 1: boot-test job on the flake supply** — `b8990c8` (ci)
2. **Task 2: gates.yml — relocated legs and the TCG quick gate** — `85701ad` (ci)

**Plan metadata:** the `docs(01-25): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `.github/workflows/xtask.yml` (modified) — all 11 `actions/checkout` references pinned to v4's sha; the boot-test job rewritten onto the flake supply; the Harvey checkout step, the `R9_HARVEY_DIR` env entry and their comment deleted; the apt provision step removed.
- `.github/workflows/gates.yml` (created) — seven jobs: the six relocated legs and `quick-gate-tcg`; header comment with the venue/witness rule and the supply-chain rule; every action sha-pinned.

## Decisions Made

- **All 11 legacy checkout pins, not just the edited job's.** actionlint reds every `actions/checkout@v3` as "the runner … is too old", so the plan's own verify (clean actionlint, any error line fails) forced the pin file-wide. Only `uses:` lines changed in the other jobs — no command, runner or timeout was touched. Recorded as a deviation below because it crosses the "leave other jobs unchanged" boundary.
- **Apt step removed, not narrowed.** The Linux devshell provides grub2, xorriso and qemu, and xtask's grub-mkrescue path is BIOS-only (`xtask/src/main.rs:4452`: "no EFI image, so no mtools"), so `mtools` is not needed either. The plan left the decision conditional exactly this way.
- **Explicit TCG where the plan asked for it, Harvey's own shape elsewhere.** tier-1 and the two x64 legs carry `R9_ACCEL=tcg`; smoke/net-smoke/cpu are invoked exactly as Harvey invoked them and select TCG themselves on a KVM-less host (no silent downgrade: TCG is the script's `auto` outcome when hvf is absent, not a requested-kvm fallback).
- **No artifact uploads.** The plan's job contract for each relocated leg is the `nix run` invocation; the retained captures remain in the job log and under `target/gates/`, and the best-effort result is the job status.
- **`permissions: contents: read` added to gates.yml.** Harvey's posture; these legs need no write scope and receive no secrets (T-01-80).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] actionlint reds the legacy `actions/checkout@v3` pins**
- **Found during:** Task 1 (running the task's own `<verify>`)
- **Issue:** `nix run nixpkgs#actionlint -- .github/workflows/xtask.yml` exited 1 with 11 errors — every `actions/checkout@v3` in all jobs, not only the boot-test job, is reported as an unsupported runner. The task's `<fails_when>` is "non-zero exit, or any reported error line", so the verify was unpassable while those pins survived, and the plan's own truth "Every third-party action is pinned by full commit sha" was unsatisfied for the file.
- **Fix:** replaced all 11 `uses: actions/checkout@v3` lines with `actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4` (the API-resolved sha Harvey's workflows already carry). No other line in those jobs changed.
- **Files modified:** `.github/workflows/xtask.yml`
- **Verification:** actionlint rc 0 with no output; `grep -E 'uses: ' | grep -vcE '@[0-9a-f]{40}'` prints 0.
- **Committed in:** `b8990c8` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking, also closing a plan-level security truth).
**Impact on plan:** The boot-test job, `gates.yml` and every command the plan specifies are exactly as written; the only extra change is the mechanical supply-chain pin in jobs the plan otherwise leaves alone, and it was required by the plan's own verify.

## Issues Encountered

- **`grep -c` exits 1 when the count is 0.** Both "<EXPECT==0>" verify commands print `0` but return status 1; the numbers, not the exit statuses, are the pass condition. Recorded here so a re-run does not read the status as a failure.
- **Two of the six referenced apps do not exist at this plan's base.** `boot-r9-net-smoke-x64` is registered by plan 01-24 (this wave) and `boot-r9-auth-x64` by plan 01-31 (next wave). The plan anticipates this: the first push carrying `gates.yml` is stop C's (plan 01-27), after 01-31, so no hosted run sees an app missing. Nothing was pushed or triggered from this worktree.

## User Setup Required

None — no external service configuration required. The workflows' first real results arrive at stop E, after the owner's push.

## Next Phase Readiness

- **The CI half of HARN-07/HARN-08 is in place; the evidence half is the backstop.** Both workflows are statically clean (actionlint) and reference only r9's flake apps. Their hosted results are recorded at stop E (D-13's best-effort second witness) — no run is possible from a worktree that must not push.
- **The relocation is complete from r9's side.** Harvey's `r9-shell.yml` and `phase6.yml` legs now all have an r9 job, so the D-10 Harvey commit can delete them without leaving a leg with no home. Harvey remains read-only and untouched by this plan.
- **STATE.md/ROADMAP.md were not modified** (parallel worktree mode; the orchestrator owns them). REQUIREMENTS.md was not ticked: HARN-07 and HARN-08 are shared with sibling plans that have no SUMMARY yet, so the shared-ID gate blocks them and the orchestrator owns the update.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created file exists on disk: `.github/workflows/gates.yml` — FOUND; `.github/workflows/xtask.yml` modified — FOUND.
- Task commits `b8990c8` (Task 1) and `85701ad` (Task 2) are ancestors of HEAD; `git rev-list --count de0c2024..HEAD` = 2 (measured against the plan ledger).
- Task 1 re-ran: actionlint rc 0, no output; Harvey-token count 0; unpinned `uses:` count 0.
- Task 2 re-ran: actionlint rc 0; six leg invocations; one quick-gate command; seven `continue-on-error: true`; unpinned `uses:` count 0.
- Plan verification re-ran: `nix run nixpkgs#actionlint -- .github/workflows/xtask.yml .github/workflows/gates.yml` rc 0; Harvey HEAD `25d97dc4d…` (unchanged, not written); zero `port/src` changes; no push.
