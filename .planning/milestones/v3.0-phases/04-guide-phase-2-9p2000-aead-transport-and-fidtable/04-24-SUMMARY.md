---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: 24
subsystem: planning-evidence
tags: [9p2000-aead, gap-closure, kernel-bytes, miri, aarch64-quick-pass, owner-stop, x86_64, boot-iso, outputHash]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
    provides: the owner-approved gap-closure port/src change at the reviewed sha 6fd201c
provides:
  - the gap-closure closing verdicts (suites twice, test-9p twice, Miri) at the reviewed commit
  - the @04-24 kernel byte evidence set that supersedes plan 04-14's voided set
  - the aarch64 quick-pass verdicts for the gap-closure kernel
  - the second owner-stop page and one-command runner for the x86_64 Linux host
affects: [04-25, phase-05]

# Actuals (#2632) — same estimateTokens scale as the plan's estimate (chars/4 over the realized diff).
actuals:
  tokens: 10428
  tasks: 2
  commits: 2
  plan_head_before: 8df21a37dc15f637337057453aa470a7d407aede
  plan_head_after: ea0e0ab7fda3877937fed7227e61cd38c808a3cd

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A later evidence set is recorded beside an earlier one and marks it superseded; the earlier files are never rewritten"
    - "An after-measurement proves its input by taking sha256 of the build output immediately before and after the metric commands"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/after-04-24.sha256
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/after-04-24.metrics
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/KERNEL-BYTES-GAPS.md
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP-2.md
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP-2-RUNNER.sh
  modified:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/closing-suite-gaps.md

key-decisions:
  - "The gap closure's kernel bytes are a second evidence set (@04-24) written beside plan 04-14's; KERNEL-BYTES-GAPS.md marks the @04-14 set superseded rather than editing it."
  - "Both ELF hashes differ from @04-14 while the defined text symbol totals are unchanged from @04-14; the page records that as measured and rests the 'fixes are linked' claim on the ELF verdict at a commit with no source diff against the reviewed sha."
  - "Miri took 357 s, over the 120-second feedback-latency budget; reported as the accepted Phase 3 deviation, not fixed (ds/ is unchanged by the gap closure)."
  - "The quick pass ran at caff8d4, a .planning-only commit on 8df21a3; the booted kernels are the ones recorded in after-04-24.sha256."

patterns-established:
  - "Second-evidence-set pages carry a three-column table (baseline, prior set, this set) so a superseding record is readable without the prior file"
  - "The second owner-stop page states the first stop's three deviations as instructions (run once; gates before any return commit; a second run is disclosed)"

requirements-completed: [WIRE-12, WIRE-13]

coverage:
  - id: D1
    description: "Both suite runs, both test-9p runs and Miri pass at the reviewed commit, with five closing PASS verdicts in closing-suite-gaps.md."
    requirement: WIRE-12
    verification:
      - kind: integration
        ref: "grep -cE '^VERDICT (suite-run-[12]|test-9p-run-[12]|miri): PASS' evidence/closing-suite-gaps.md -> 5"
        status: pass
    human_judgment: false
  - id: D2
    description: "The @04-24 kernel byte set: both kernels rebuilt from nothing, measured file proven by sha256 before and after, four baseline METRIC-COMMAND lines unchanged, elf DIFFERENT, fpsimd EQUAL, x86_64 text under 2 MiB."
    requirement: WIRE-13
    verification:
      - kind: other
        ref: "MEASURED sha256 == after-04-24.sha256 elf lines (2/2); METRIC-COMMAND lines identical to before.metrics and naming the build output (4/4); fpsimd EQUAL verdicts 2/2; text-under-2mib PASS 1/1; git diff --quiet 8df21a3 HEAD over kernel inputs -> rc 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "The aarch64 quick pass: tier-1 on HVF, tier-1 on TCG, smoke, net-smoke and cpu each exit 0 with an ALL STAGES PASS line."
    requirement: WIRE-12
    verification:
      - kind: e2e
        ref: "target/phase04-closing2/quick/*.log: five runs rc=0, each with ALL STAGES PASS"
        status: pass
    human_judgment: false
  - id: D4
    description: "The second owner-stop page and runner: ordered checklist, hole-in-one runner, bash syntax clean, native TCG venue stated, hash commit restricted to flake.nix, returned-round guard."
    requirement: WIRE-13
    verification:
      - kind: other
        ref: "bash -n 04-OWNER-STOP-2-RUNNER.sh -> rc 0; TCG gate and branch named in both files; flake.nix unchanged"
        status: pass
    human_judgment: true
    rationale: "Whether the page is operable by the owner on an unfamiliar host is a human judgement; the mechanical checks only prove the text and the script are well formed."

duration: 11 min
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 24: Closing evidence on the Mac and the second owner-stop page Summary

**The gap-closure kernel now has its own evidence set at the owner-reviewed commit — suites and `test-9p` twice, Miri, both kernels rebuilt from nothing, and the aarch64 quick pass green — plus the x86_64 Linux checklist and runner for the second owner stop.**

## Performance

- **Duration:** 11 min (14:18:47Z to 14:29:22Z)
- **Started:** 2026-11-25T14:18:47Z
- **Completed:** 2026-11-25T14:29:22Z
- **Tasks:** 2/2
- **Files modified:** 6 tracked files (2 modified/new evidence plus 3 new artefacts and the closing-suite append; 665 inserted lines)

## Accomplishments

- **Ten closing PASS verdicts at the reviewed commit.** Both full suite runs (four commands each), both `test-9p` runs and Miri are green at `8df21a3`, a commit with no source diff against the reviewed sha `6fd201c`; `MIRI-TIME closing 357` is recorded as the accepted Phase 3 over-budget deviation.
- **The `@04-24` kernel byte evidence set replaces plan 04-14's voided set.** Both kernels rebuilt from nothing at `8df21a3`: `virtshell` `2ee30a2e…27dcee`, `q35churnshell` `2066521c…f24928`. Both whole-ELF hashes are `DIFFERENT` from `@04-14`, both FP/SIMD counts are `EQUAL` to the baseline (309 / 6), and the x86_64 text total stays under 2 MiB.
- **The after-measurement is provably the after-build.** `after-04-24.metrics` carries the four `METRIC-COMMAND` lines of `before.metrics` unchanged, each naming the repo-relative build output; the sha256 of each measured file taken immediately before and after the commands is identical and equals that config's `elf` line.
- **The aarch64 quick pass is green for the gap-closure kernel.** `quick-tier1-hvf`, `quick-tier1-tcg`, `quick-smoke`, `quick-net-smoke` and `quick-cpu` each exit 0 with an `ALL STAGES PASS` line; the full HVF series is not run (D-25).
- **The second owner stop is ready.** `04-OWNER-STOP-2.md` is the ordered x86_64 Linux checklist (host tests before the hash step, the sandboxed builds, the owner's `flake.nix`-only `outputHash` commit, KVM, native TCG under D-25, the full set, the report) and `04-OWNER-STOP-2-RUNNER.sh` is the one-command runner with the returned-round guard.

## Text symbol totals and headroom

| Config | Baseline (`3ce5ed8`) | `@04-14` (`3398df4`) | `@04-24` (`8df21a3`) |
|--------|----------------------|----------------------|----------------------|
| `virtshell` text symbol bytes | 754232 | 755220 | 755220 |
| `q35churnshell` text symbol bytes | 759072 | 760137 | 760137 |
| `virtshell` FP/SIMD scan lines | 309 | 309 | 309 |
| `q35churnshell` FP/SIMD scan lines | 6 | 6 | 6 |

The x86_64 text symbol total of **760,137 bytes leaves 1,337,015 bytes of headroom** under 2,097,152 bytes (2 MiB). The defined text totals are unchanged from `@04-14` while the whole-ELF hashes differ; the evidence page records that as measured, without interpreting it.

## Task Commits

Each task was committed atomically:

1. **Task 1: closing suites twice, Miri, and the kernel bytes of the gap closure** — `caff8d4` (evidence)
2. **Task 2: the aarch64 quick pass and the second owner-stop page with its runner** — `ea0e0ab` (evidence)

**Plan metadata:** the SUMMARY commit (docs: complete 04-24 plan)

_Measured from the plan ledger: 2 commits between `8df21a3` and `ea0e0ab`, excluding the SUMMARY commit._

## Files Created/Modified

- `evidence/closing-suite-gaps.md` — appended the five closing suite/Miri verdicts and the five quick-pass verdicts, each at the HEAD it measured
- `evidence/kernel-bytes/after-04-24.sha256` — the four-line after record (`after 8df21a3…`, venue, both ELF hashes)
- `evidence/kernel-bytes/after-04-24.metrics` — four numeric lines, the four baseline `METRIC-COMMAND` lines unchanged, two `MEASURED` lines
- `evidence/kernel-bytes/KERNEL-BYTES-GAPS.md` — the superseding page with the three-column table and the seven `@04-24` verdict lines
- `04-OWNER-STOP-2.md` — the second owner-stop checklist (branch `phase-04/aead-wire-gaps`, logs `~/r9-p04-stop2/`, return `evidence/owner-stop-2/return/`)
- `04-OWNER-STOP-2-RUNNER.sh` — the fail-fast runner, `bash -n` clean, with the `RERUN=1` returned-round guard

## Decisions Made

- **Second evidence set, not a rewrite.** `KERNEL-BYTES-GAPS.md` marks the `@04-14` set superseded and states why (every gap fix under `port/src` changes kernel bytes); `after-04-14.sha256`, `after.metrics`, `KERNEL-BYTES.md` and `closing-suite.md` are untouched.
- **The identical text totals are recorded, not explained away.** The page states the ELF verdict is the proof the fixes are linked, because the sum of defined `T`/`t` symbol sizes did not move.
- **The quick pass ran at `caff8d4`.** That is a `.planning`-only commit over the measured `8df21a3`; the kernels booted are the ones in `after-04-24.sha256`, and the evidence names the HEAD.
- **Miri 357 s is reported, not fixed.** The plan forbids source edits and `ds/` is unchanged by the gap closure; this is the accepted Phase 3 deviation.

## Deviations from Plan

None - plan executed exactly as written. No stop rule fired: both `fpsimd` verdicts are `EQUAL`, `text-under-2mib-q35churnshell@04-24` is `PASS`, and every suite, `test-9p` and Miri run exited 0.

## Issues Encountered

- **Miri is over the 120-second feedback-latency budget** (`MIRI-TIME closing 357`). Handled as the plan directs: reported in the evidence, no coverage cut, no source edit. It is the same closing check plan 04-14 made, and `ds/` is unchanged by the gap closure.

## User Setup Required

None for this plan. The owner's next step is the second owner stop (`04-OWNER-STOP-2.md`) on the x86_64 Linux host, after the branch is pushed on approval; that page is plan 04-25's input.

## Next Phase Readiness

- Plan 04-25 can present `04-OWNER-STOP-2.md` and its runner as written; nothing in either file needs a Mac-side follow-up.
- The committed `boot-iso` `outputHash` is expected **not** to match the gap-closure x86_64 kernel; the owner re-derives it on the Linux host.
- No source file, `Cargo.lock`, `flake.nix` or Rust toolchain input changed in this plan; `origin/main` stays `5d1549a` and nothing is pushed.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

Every file this plan names exists on disk (`04-24-SUMMARY.md`, `evidence/closing-suite-gaps.md`,
`evidence/kernel-bytes/after-04-24.sha256`, `after-04-24.metrics`, `KERNEL-BYTES-GAPS.md`,
`04-OWNER-STOP-2.md`, `04-OWNER-STOP-2-RUNNER.sh`), both plan commits (`caff8d4`, `ea0e0ab`) are
ancestors of HEAD, the ten closing PASS verdicts, the seven `@04-24` verdict lines, the
`bash -n` runner check and the `flake.nix`-unchanged check all pass, and `origin/main` is still
`5d1549af07de7220d6f48aac0d0db221aa7c541c`.
