---
phase: 01-stage-p0-cross-build-harness
plan: 03
subsystem: infra
tags: [qemu, q35, harvey, 9k, k10cpu, tcg, kvm, spike]

# Dependency graph
requires: []
provides:
  - "s5/boot-q35.sh: launch-and-classify harness for Harvey's own 9k kernel (TCG or KVM, q35 or pc, virtio/none/default)"
  - "Mac TCG evidence: three PASS legs for k10cpu.elf under q35+virtio, q35 no-NIC, and the pc baseline"
  - "kernel provenance: sha256 ecdcdad... and an explicit unknown-source-commit statement"
  - "build-path/prompt/q35 analysis with file:line citations at bd7a6982a and two D-20 scope options"
  - "KVM witness command and kernel sha256 for Plan 06"
affects: [01-06, phase-2-planning]

actuals:
  tokens: 10211
  tasks: 2
  commits: 2
plan_head_before: 3ec688392c41a7c6114617903c21de4f8883b7bc
plan_head_after: 905c5fb

tech-stack:
  added: []
  patterns:
    - "Capture-before-start: truncate the serial log, launch qemu with -serial file:, classify the whole capture"
    - "KVM refusal over silent downgrade: S5-ACCEL=kvm requires writable /dev/kvm or exits S5-ERROR kvm-unavailable"
    - "Read-only Harvey access: cp the kernel out, git show sources at the pin, never run inside the checkout"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/boot-q35.sh
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/.gitignore
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/evidence/kernel-provenance.txt
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/evidence/q35-virtio-tcg-mac.log
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/evidence/q35-nonic-tcg-mac.log
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/evidence/pc-baseline-tcg-mac.log
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/evidence/build-path-analysis.md
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/S5-FINDINGS.md
  modified: []

key-decisions:
  - "The Mac kernel copy's source commit is unknown (no build path, date or revision string survives); the KVM leg pins the exact sha256 instead"
  - "One script serves the Mac TCG matrix and the owner's KVM witness; KVM never falls back to TCG silently (D-13)"
  - "D-20 minimum scope supported by evidence: build and boot to the root is from prompt; the rc prompt via a host-served u9fs root is the next increment"
  - "The pc baseline reproduces the recorded 2026-11-05 behaviour, not the exact artifact (the kernel was rebuilt on 2026-11-18)"

patterns-established:
  - "S5 harness contract: BANNER/PROMPT/SIGS/NIC lines plus a final S5-RESULT machine=... line"
  - "Whole-log signature classification (bad magic, panic, suicide: sys: trap) with the prompt as the healthy marker"

requirements-completed: [HARN-08]

coverage:
  - id: D1
    description: "boot-q35.sh launches a 9k10cpu.elf copy under q35 or pc, TCG or KVM, with virtio/none/default networking and a kill/reap trap"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- spikes/s5/boot-q35.sh (exit 0, no findings)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Three Mac TCG legs PASS on the same bytes, each with a serial excerpt and an S5-RESULT line"
    requirement: HARN-08
    verification:
      - kind: integration
        ref: "grep -c 'S5-RESULT machine=' on the three evidence logs (1 each); VERDICT lines in S5-FINDINGS.md (3)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Read-only build-path, prompt and q35 analysis with citations, plus two D-20 scope options"
    requirement: HARN-08
    verification:
      - kind: other
        ref: "grep -cE '^## ' build-path-analysis.md (5 sections) and the file:line citation table"
        status: pass
    human_judgment: true
    rationale: "Adequacy of the analysis and the choice between D-20 option A and B need owner review at the closing checkpoint"
  - id: D4
    description: "KVM witness leg on the owner's x86_64 Linux host against the same kernel sha256"
    requirement: HARN-08
    verification: []
    human_judgment: true
    rationale: "The Mac cannot run KVM; Plan 06 must run boot-q35.sh with S5_ACCEL=kvm on the owner's host"

duration: 3 min
completed: 2026-11-23
status: complete
---

# Phase 1 Plan 3: S5 — Harvey's own kernel under QEMU q35 Summary

**Harvey's own 9k10cpu kernel boots to the `root is from (tcp)[tcp]:` prompt under `-M q35` on the Mac TCG matrix, with a reusable TCG/KVM harness and a cited build/prompt analysis for D-20.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-11-23T00:41:45Z
- **Completed:** 2026-11-23T00:45:13Z
- **Tasks:** 2
- **Files modified:** 8 (all created; `S5-FINDINGS.md` extended in Task 2)

## Accomplishments

- `s5/boot-q35.sh` launches a byte-identical copy of Harvey's `amd64/9k10cpu.elf` under q35 or pc, TCG or KVM, with virtio/none/default networking, capture-before-start, whole-log signature classification, and a kill/reap trap on every exit path. `S5_ACCEL=kvm` without a writable `/dev/kvm` exits `S5-ERROR kvm-unavailable`; there is no silent TCG downgrade (D-13). shellcheck-clean.
- Three Mac TCG legs PASS on the same kernel bytes: q35 + virtio-net, q35 with no NIC, and the pc baseline. All reach `root is from (tcp)[tcp]:` with no `bad magic`/`panic`/`suicide: sys: trap` signature. The pc leg reproduces the recorded 2026-11-05 behaviour.
- Kernel provenance is recorded: sha256 `ecdcdaddea8d3759d74303b869ddfae8efeba1d4d160382577cfc52b9a32c45b`, 2030226 bytes, source mtime 2026-11-18T12:34:43Z. No build path or revision string proves the source commit, and the file says so plainly.
- `s5/evidence/build-path-analysis.md` answers the four D-20 questions with file:line citations at `bd7a6982a`: the setup/toolchain/build chain and its sudo/namespace needs on Linux and darwin (including the passwordless `unshare -Ur -m` venue), the tcp-only boot method and what an rc prompt needs, q35 NIC/PCI specifics, and two scope options.
- `S5-FINDINGS.md` carries the three Mac verdicts, `VERDICT s5-q35-kvm-linux: PENDING-OWNER` with the exact Plan 06 command and sha256, `ASSUMPTION A11: UNTESTED` for the cpu-service part D-20 no longer requires, and the pass-2 implications.

## Task Commits

Each task was committed atomically:

1. **Task 1: boot-q35.sh plus the Mac TCG matrix** — `cfdafcd` (feat)
2. **Task 2: read-only build and prompt analysis, D-20 scope options, pending KVM leg** — `905c5fb` (docs)

**Plan metadata:** committed with this SUMMARY (`docs(01-03)`).

## Files Created/Modified

- `spikes/s5/boot-q35.sh` — q35/pc launch-and-classify harness for Harvey's own kernel, TCG or KVM
- `spikes/s5/.gitignore` — ignores `work/` (kernel copy, raw logs, harness stdout)
- `spikes/s5/evidence/kernel-provenance.txt` — sha256, size, mtime, file(1), strings hints, unknown-commit statement
- `spikes/s5/evidence/q35-virtio-tcg-mac.log` — harness stdout + serial capture, q35/virtio
- `spikes/s5/evidence/q35-nonic-tcg-mac.log` — harness stdout + serial capture, q35/no NIC
- `spikes/s5/evidence/pc-baseline-tcg-mac.log` — harness stdout + serial capture, pc/default NIC baseline
- `spikes/s5/evidence/build-path-analysis.md` — build path, prompt semantics, q35 specifics, D-20 options
- `spikes/S5-FINDINGS.md` — S5 verdicts, evidence lines, KVM pending leg, A11, pass-2 implications

## Decisions Made

- **D-20 minimum scope:** the evidence supports "build and boot to the `root is from` prompt" as the one Harvey command. The rc-prompt option is feasible without a Harvey source edit via Harvey's own u9fs on TCP 564, but it adds a privileged/userns listener and prompt driving, so it is the next increment.
- **Kernel provenance is a sha256, not a commit:** the ELF embeds no build path or revision. Plan 06's KVM leg must match the recorded sha256; a freshly built kernel at `bd7a6982a` gets its own record.
- **One launch path:** the same script will run the Mac convenience legs and the required KVM witness, so the classification cannot drift between venues.
- **Boot prompt precedes networking:** all three legs reach the prompt because `rootserver` runs before method configuration; this is why the no-NIC leg passes, and it means a tcp root is unreachable without a NIC.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Enabled the documented protected-branch override for sequential execution on `main`**
- **Found during:** Task 1 commit
- **Issue:** The pre-commit head assertion refuses commits on the default branch. This repo's `branching_strategy` is `none` and the orchestrator dispatches sequential executors on `main`; without the override no task commit is possible.
- **Fix:** Set `git.allow_default_branch_commits: true` in `.planning/config.json`, the override the guard's own error message names. The file is runtime config and remains untracked; it is not part of this plan's deliverables.
- **Files modified:** `.planning/config.json` (untracked)
- **Verification:** `gsd_run query git.base-branch --is-protected main` prints `false`; both task commits succeeded on `main`.
- **Committed in:** n/a (config, not committed)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Environment/config only; no production or planning content changed by the deviation.

## Issues Encountered

None. The `pc` kernel was rebuilt on 2026-11-18 (after the recorded 2026-11-05 pool-panic run), so the baseline reproduces the recorded behaviour, not the exact artifact; this is documented in the provenance and findings rather than treated as a discrepancy.

## User Setup Required

None — no external service configuration required. Plan 06 needs the owner's x86_64 Linux host with a writable `/dev/kvm`; the exact command and kernel sha256 are in `S5-FINDINGS.md`.

## Next Phase Readiness

- The Mac TCG matrix is complete with serial evidence; the harness is ready for the KVM witness.
- Plan 06 must run the recorded command against the recorded sha256 (or a fresh `bd7a6982a` build with its own sha256) and replace `PENDING-OWNER` with PASS/FAIL.
- The closing checkpoint can choose D-20 option A (boot prompt) or option B (rc prompt via u9fs) using the analysis; option A is the evidence-backed minimum.
- Harvey and r9's protected paths are untouched; no pushes were made.

## Self-Check: PASSED

- All eight created files exist on disk.
- Both task commits (`cfdafcd`, `905c5fb`) are ancestors of HEAD.
- All task acceptance criteria and the plan-level verification (Harvey untouched, protected paths clean, no pushes) passed.
