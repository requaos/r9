---
phase: 01-stage-p0-cross-build-harness
plan: 28
subsystem: cross-build-harness
tags: [flake-lock, harvey-pin, witness, stop-e, kvm, q35, phase-boundary-audit, harn-01, harn-02, harn-06, harn-07, harn-08]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-27/01-34's approved, applied and pushed Harvey flip at c697666; the stop-C2/C3 green moved set; the 01-18 gate plus 01-33's driver fix; the 01-08/01-11 image engine and manifests; the 01-10/01-13/01-14 flake-supplied trees; 01-30's aarch64 quick pass"
provides:
  - "flake.lock: harvey locked at the pushed flip commit c697666ba315f137ca2fd11768f835d04bd9f018, no nested harvey/r9 node, no 116bcc2"
  - "witness/WITNESS-LOG.md: the Stop E section (eleven PASS verdicts, the q35 result, the CI disposition) and the Phase boundary audit (R1-R7 evidence map, prohibition audit)"
  - "witness/evidence/stop-e/: trimmed, redacted stop-E evidence (logs, tables, q35 paste, CI note, secret scan)"
  - "harvey-flip/HARVEY-FLIP.md: the stop-E closure for the pushed flip commit"
affects: [phase-verification, 02, HARN-08, HARN-01, HARN-02, HARN-06, HARN-07]

actuals:
  tokens: 20773
  tasks: 3
  commits: 3

plan_head_before: 3283797bc356bb6d90e7d463ab6dbd6cce5921a8
plan_head_after: 54b71a7c378fd49b057d1e69abe7db36c93258d4

tech-stack:
  added: []
  patterns:
    - "The pin bump is lock-only: the Harvey rev is checked on GitHub first, then locked, then the lock is asserted to have no nested r9 node and no legacy 116bcc2"
    - "Stop-E evidence is trimmed to the decisive lines (≤200 lines per file) and carries its own credential scan at 0"
    - "The phase-boundary audit maps R1-R7 to verdict ids, commit shas and file paths, and records each prohibition with the command and result"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/trees.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/kernel.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/gate.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/gate-neg.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/profiles.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/cpuauth-ref.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/full.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/full.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/soak.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/stress.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/equiv/report.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/store-verify.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/summary.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/env.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/harvey-q35.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/ci.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/secret-scan.txt
  modified:
    - flake.lock
    - .planning/phases/01-stage-p0-cross-build-harness/witness/WITNESS-LOG.md
    - .planning/phases/01-stage-p0-cross-build-harness/harvey-flip/HARVEY-FLIP.md

key-decisions:
  - "The Harvey pin is the pushed flip commit c697666ba315f137ca2fd11768f835d04bd9f018, verified on origin/master before locking; the bump commit 626c80c changes flake.lock only (D-06, D-23)."
  - "The stop-E verdicts use the final full run (623461-1795468943, 8/8); the return's earlier full.tsv (which recorded a tier1 FAIL) is disclosed, not dropped."
  - "The Return's env harvey_pin=bd7a698 is the driver's legacy-final equivalence baseline, not the r9 lock; the lock is c697666 and the trees are stamped with it."
  - "Hosted CI is informational: xtask.yml is green and gates.yml is green except the best-effort quick-gate-tcg (D-13); the owner's native amd64 TCG gate is GATE-PASS at stop C3."
  - "The audit compares kernel bytes at the bump (stop-E ELF 2377dc2f equals stop-C3 2377dc2f), not to the stale 01-08 x86_64 baseline, because the approved 01-33/01-34 x86_64 fixes changed x86_64 source."

patterns-established:
  - "A lock-only pin bump is audited by the commit's file list, the lock's node graph, the pushed rev and the tree stamps (rev=c697666 on both trees)."
  - "A phase-boundary audit records evidence, not intent: each R row names verdict ids and paths; each prohibition row names the command and result."

requirements-completed: [HARN-08, HARN-01, HARN-02, HARN-06, HARN-07]

coverage:
  - id: D1
    description: "r9 locks the pushed Harvey flip commit c697666 in a lock-only commit; flake.lock has no nested r9 node and no 116bcc2"
    requirement: HARN-08
    verification:
      - kind: automated
        ref: "jq -r '.nodes.harvey.locked.rev' flake.lock; jq -r '.nodes.harvey.inputs | keys[]'; grep -c 116bcc2 flake.lock; git ls-remote requaos/harvey refs/heads/master"
        status: pass
    human_judgment: false
  - id: D2
    description: "Owner stop E at the bump: eleven KVM steps green, the full moved set 8/8, the gate in both directions, nine profiles, soak/stress, equivalence 10/10 and the tree verify"
    requirement: HARN-01
    verification:
      - kind: manual_procedural
        ref: ".planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/summary.tsv + WITNESS-LOG.md '## Stop E'"
        status: pass
    human_judgment: true
    rationale: "The witness ran on the owner's x86_64 KVM host; the agent cannot reach that host or push r9."
  - id: D3
    description: "Harvey's own-kernel command at the pushed commit prints the root prompt and a kernel sha256"
    requirement: HARN-08
    verification:
      - kind: manual_procedural
        ref: ".planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/harvey-q35.log"
        status: pass
    human_judgment: true
    rationale: "The q35 build runs in the owner's Harvey checkout through lima/sudo; the paste is the record."
  - id: D4
    description: "The phase-boundary audit maps R1-R7 to evidence and re-checks every phase prohibition at the bump"
    requirement: HARN-08
    verification:
      - kind: automated
        ref: "grep -c '^## Phase boundary audit' WITNESS-LOG.md; for r in R1..R7 grep '^| $r |'; the prohibition commands in the audit table"
        status: pass
    human_judgment: true
    rationale: "The judgment that each mapping is sufficient is an audit reading, even though every command in it is reproducible."

duration: 80min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 28: The lock-only pin bump, owner stop E, and the phase-boundary audit — Summary

**Harvey is locked at the pushed flip commit `c697666` in a lock-only bump; the owner's stop-E KVM witness at `af46615` is 11/11 green, Harvey's own q35 kernel boots to `root is from`, and the R1-R7 audit maps every phase truth to its evidence with all prohibitions re-checked.**

## Performance

- **Duration:** ~80 min wall (Task 1's bump 2026-11-23T20:34Z; the owner's stop-E session 20:59Z–21:25Z; Task 3 records 21:54Z)
- **Started:** 2026-11-23T20:34:41Z
- **Completed:** 2026-11-23T21:54:31Z
- **Tasks:** 3 (Task 2 is the owner's stop-E checkpoint, not agent work)
- **Files modified:** 21 (including this SUMMARY)

## Accomplishments

- **The flip is complete and pinned.** `flake.lock` locks Harvey `c697666ba315f137ca2fd11768f835d04bd9f018`, the pushed flip commit; the nested `harvey/r9` node and its duplicate nixpkgs/rust-overlay nodes are gone; `116bcc2` appears nowhere in the lock. The bump `626c80c` (merged as `af46615`) changes `flake.lock` only, and `git ls-remote` shows the rev on `origin/master`.
- **Stop E is 11/11 green at the bump.** `trees` (both trees at `rev=c697666`, `TREES-VERIFY-OK`), `soak` (tap venue, 9 tcpblast, ALL STAGES PASS), `kernel` (`r9-kernel-x64-af46615`, ELF `2377dc2f…`, boot-iso BUILD+REBUILD OK), `gate` (two runs, both exit 0, `GATE-PASS rio-serving` + `GATE-PASS control-no-rio`, ALL STAGES PASS), `gate_neg` (`GATE-FAIL rio-not-serving` → `GATE-NEG-EXPECTED-FAIL`), `profiles` (nine exit 0), `cpuauth_ref` (cpu 4442, auth 95 both match), `full` (8/8), `stress` (tap venue, 9 tcpblast, ALL STAGES PASS), `equiv` (all 10 rows match) and `store_verify` (both trees `STORE-VERIFY-OK`).
- **Harvey's own kernel works from the pushed commit.** The owner's `nix run .#q35` paste records `Q35-KERNEL … sha256=ecdcdadd… bytes=2030226`, `Q35-PROMPT root is from (tcp)[tcp]:`, `Q35-SIGS none` and `Q35-RESULT machine=q35 accel=tcg banner=yes prompt=yes sigs=none`.
- **The phase boundary is audited.** R1-R7 are all PASS with their verdict ids, commits and paths; the prohibition audit re-checks the pushed pin, the one-commit Harvey flip with its 11 untouched font paths, the lock-only bump, the absence of a `port/src` edit, the gate and D-28 marker, the manifest Rust-entry absence, the trees and the kernel bytes. Residual risks are recorded in the ledger.
- **CI is recorded as the best-effort second witness.** `xtask.yml` is all green; `gates.yml` is green except `quick-gate-tcg` (red). The owner's native amd64 TCG gate is `GATE-PASS` at stop C3.

## Task Commits

Each task was committed atomically:

1. **Task 1: Verify the Harvey push, bump the pin lock-only, and check the bump on the Mac** — `626c80c` (build; `flake.lock` only), merged as `af46615` (chore)
2. **Task 2: Owner stop E** — no commit (the owner's human-action checkpoint; the return is `95af737` on `witness-returns/01-28`)
3. **Task 3: Record stop E and write the phase-boundary audit** — `54b71a7` (docs)

**Plan metadata:** the final SUMMARY commit (docs: complete plan)

## Files Created/Modified

- `flake.lock` — Harvey at `c697666`; no nested r9; no `116bcc2` (Task 1, `626c80c`).
- `.planning/phases/01-stage-p0-cross-build-harness/witness/WITNESS-LOG.md` — the `## Stop E` section (eleven verdicts, q35, CI) and `## Phase boundary audit` (R1-R7 table, prohibition table, residual risks).
- `.planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/` — 17 files: the trimmed logs/tables for trees, kernel, gate, gate-neg, profiles, cpuauth-ref, full, soak, stress, equiv, store-verify, summary, env; the q35 paste; the CI note; the secret scan. Every log is ≤200 lines; the credential scan is 0.
- `.planning/phases/01-stage-p0-cross-build-harness/harvey-flip/HARVEY-FLIP.md` — the stop-E closure: the bump, the 11/11 witness, the q35 result, the CI disposition and the audit pointer.

## Decisions Made

- **Lock-only means lock-only.** `626c80c` touches `flake.lock` and nothing else; the merge `af46615` carries no source change. The audit confirms the rev is pushed before it is locked (D-23).
- **The final full run is the verdict.** The return holds three `full.tsv` snapshots; the first recorded `boot-r9-tier1-x64 FAIL`, the second and final are 8/8. The final run's `GATE PASS full … run=623461-1795468943` is the stop-E verdict, and the earlier snapshot stays in the return tree and is disclosed.
- **`env.txt`'s `harvey_pin` is a baseline, not the lock.** `bd7a6982…` is `run-witness.sh`'s legacy-final pin for the Harvey-hosted equivalence captures; the lock is `c697666` and both trees are stamped with it.
- **The bounded audit does not launder the stale plan text.** Two plan literals were superseded by later approved work: the 01-08 x86_64 kernel baseline (the 01-33/01-34 x86_64 fixes changed x86_64 source) and the `password=` regex (it matches parser contract lines, not values). Both are recorded as audit notes, with the intent proven directly.
- **STATE.md and ROADMAP.md are untouched.** The orchestrator asked for the plan records only; phase verification runs next.

## Deviations from Plan

### Auto-fixed Issues

None — no code fix was needed. The plan's implementation (Task 1) was already complete when this execution began; Task 3 is records and audit.

### Documented plan-text supersessions

**1. The 01-08 x86_64 kernel baseline no longer applies**
- **Found during:** Task 3 audit (P13)
- **Issue:** The plan's Task 1 acceptance says "kernel ELF sha256 values equal the baseline"; the 01-08 baseline x86_64 hash is `50b01c9a…`. The approved 01-33/01-34 x86_64 fixes (`9af05f8`, `2a5cbed`) changed x86_64 source, so the current devshell build is `8f2d485e…` (reproduced twice), and the aarch64 build still matches `76e6e153…`.
- **Resolution:** The audit compares at the bump — stop-E kernel ELF `2377dc2f…` equals stop-C3 `2377dc2f…`, and the boot-iso BUILD+REBUILD is OK. The bump itself changed no kernel source. This is recorded in P13, not hidden.
- **Files:** `witness/WITNESS-LOG.md` P13; `witness/evidence/stop-e/kernel.log`.
- **Committed in:** `54b71a7`.

**2. The plan's `password=` regex is over-broad**
- **Found during:** Task 3 audit (P10)
- **Issue:** The plan's automated check `git diff <01-08 plan_head_before> HEAD | grep '^+' | grep -cE 'password=[^<`"[:space:]]'` expects 0 but printed 20 at `af46615` (before this audit's own row; 21 once this row is counted). All 20 code/evidence lines are benign: the credentials parser contract (`grep -q '^password='`, `sed 's/^password=//p'`), runtime creds writers (`printf 'password=%s'`), synthetic `FAKEPW123451` Rust fixtures, and doc refs.
- **Resolution:** The audit records the literal result and classifies every match, then proves the intent: no credential value is committed, `git ls-files | grep -cE '\.creds$|auth-creds'` is 0, and the stop-E evidence scan is 0. The check measured presence of the token, not a secret.
- **Files:** `witness/WITNESS-LOG.md` P10; `witness/evidence/stop-e/secret-scan.txt`.
- **Committed in:** `54b71a7`.

**3. The aarch64 bump log is not in the stop-E return**
- **Found during:** Task 3 audit (R5)
- **Issue:** The stop-E set is x86_64 KVM; the aarch64 quick pass at the bump was a Task 1 Mac check reported green by the preceding executor, but no log is retained in this plan's artifacts.
- **Resolution:** R5 records the 01-30 Mac log (`AARCH64-QUICK-PASS`, `target/aarch64-quick-pass.log` in the 01-30 record) and the Task 1 bump report, and lists the missing bump log as a residual risk. The aarch64 kernel still hashes to the 01-08 baseline.
- **Files:** `witness/WITNESS-LOG.md` R5 and Residual risks.
- **Committed in:** `54b71a7`.

**4. R4's swap proof predates the 01-34 gate fix**
- **Found during:** Task 3 audit (R4)
- **Issue:** The `PROBE-FLIP-SHA` / `PROBE-FLIPBACK-SHA` KVM run is stop B3; the 01-34 gate fix changed `gate.rs` afterwards (240 additions, 1 deletion that is a log line).
- **Resolution:** The swap assertion is unchanged and the manifests have no Rust entry; the stop-B3 probe/flipback PASS and the 01-20 member-list proof remain the evidence. Recorded as a residual risk rather than re-flipping the manifest.
- **Files:** `witness/WITNESS-LOG.md` R4 and Residual risks.
- **Committed in:** `54b71a7`.

---

**Total deviations:** 0 auto-fixed; 4 documented plan-text supersessions/residuals.
**Impact on plan:** The phase's claims remain evidence-backed; no code was changed to make an audit pass.

## Issues Encountered

- **The first full iteration failed.** `out/full/347497-1795455456/full.tsv` records `boot-r9-tier1-x64 FAIL`; the owner re-ran and the second and final runs are 8/8. The final run is the verdict and the earlier snapshot is disclosed. Stop C3's full run was also 8/8.
- **The return's `env.txt` names a different Harvey pin than the lock.** It is the driver's `legacy-final` equivalence baseline (`bd7a6982…`), not the r9 pin; the lock and both tree stamps are `c697666`. Explained in the Stop E section.
- **The hosted CI TCG leg is red.** `quick-gate-tcg` is red on the hosted runner; D-13 makes it best-effort, and the owner's native amd64 TCG gate is `GATE-PASS` at stop C3. Recorded, not waived silently.

## Authentication Gates

- **Task 2 was an owner `checkpoint:human-action` gate** (plan task type), as designed: the executor cannot push r9, reach the KVM host, or run Harvey's sudo/lima build. The owner completed the push, the KVM stop E, the q35 command and the CI report; the return is `witness-returns/01-28` at `95af737`. This is normal plan flow, not a deviation.

## Known Stubs

None. The evidence files are complete for the selected steps; the audit's residual risks are disclosed, not stubbed.

## Threat Flags

None. The new files are read-only evidence under `.planning/`; no network endpoint, auth path, schema or trust-boundary surface was added. The plan's T-01-90 (unpushed rev) is mitigated by the `git ls-remote` check, T-01-91 (non-lock changes) by the one-file commit, and T-01-92 (evidence disclosure) by the redaction and the 0 credential scan.

## Phase Boundary Audit Outcome

| Truth | Verdict | One-line evidence |
|---|---|---|
| R1 | PASS | `stope-gate` (both directions, exit 0) + `stope-gate-neg` (`GATE-NEG-EXPECTED-FAIL`) at `af46615` |
| R2 | PASS | 14/14 resolver tests; both trees `STORE-VERIFY-OK` after the full set |
| R3 | PASS | manifests per arch; both drift tests pass; all eight images assemble and boot |
| R4 | PASS | stop-B3 probe/flipback at the recorded swap shas; no Rust manifest entry remains |
| R5 | PASS | nine q35 profiles exit 0 and full 8/8 at the bump; aarch64 at 01-30 plus the bump report |
| R6 | PASS | `inputs.harvey`; lazy per-arch devshell exports; `R9_HARVEY_DIR` never exported; trees at `c697666` |
| R7 | PASS | r9 owns the 20 composed apps; Harvey `c697666` has only `nixpkgs`; every moved gate passes from r9 |

**Residual risks:** hosted `quick-gate-tcg` red (best-effort, native TCG PASS); the first full iteration's tier1 FAIL before two 8/8 retries; R4's proof predates the 01-34 gate fix; no bump-time aarch64 log; the 01-08 x86_64 kernel baseline superseded by the approved 01-33/01-34 x86_64 fixes. Details in `witness/WITNESS-LOG.md` "### Residual risks and dispositions".

## Next Phase Readiness

- The Harvey flip is complete and pinned; the phase's seven ROADMAP truths are mapped to witnessed evidence.
- The orchestrator runs phase verification next. STATE.md and ROADMAP.md were deliberately not modified by this execution.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Created files: all 17 `witness/evidence/stop-e/*` paths plus this SUMMARY exist.
- Commits: `626c80c` (bump), `af46615` (merge) and `54b71a7` (stop-E records) are ancestors of HEAD.
- Audit: `grep -c '^## Phase boundary audit'` = 1; `| R1 |` through `| R7 |` all present; `R1-R7-MAPPED` printed.
- Evidence: the stop-E credential scan is 0; every evidence file is ≤200 lines except `cpuauth-ref.log` at 152 (also ≤200).

