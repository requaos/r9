---
phase: 01-stage-p0-cross-build-harness
plan: 34
subsystem: cross-build-harness
status: complete
phase_status: "Every witness step is green and every owner approval is recorded: stop C2 full 8/8 with fpprobe and tap stress; stop C3 soak/equiv/kernel/store_verify; the standalone amd64 TCG gate GATE-PASS (owner report); the HARN-08 equivalence approved (tier1 39, auth 10, churn 1, stress tap venue); the Harvey diff approved and applied as Harvey c697666ba315f137ca2fd11768f835d04bd9f018 (36 paths, +285/−9972, zero font paths), UNPUSHED. The only remaining step is the owner's Harvey push."
tags: [witness, stop-c2, stop-c3, x86_64, kvm, tcg, fpprobe, notes, timer, tap, equivalence, harvey-flip, harn-02, harn-08]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-27's stop-C record and evidence; 01-26's full/soak/stress/equiv/store_verify steps; 01-31's x86_64 apps; 01-33's green stop-B3 and fixed driver"
provides:
  - "witness/run-witness.sh: the r9-tree tap seam (HARVEY + HARVEY_NETADDR_SEAM), R9_NET=tap for soak/stress, the full-only capture loop, and the `recorded` equivalence source"
  - "xtask/src/gate.rs: FULL_APP_OUT_DIRS_* + TAP_APP_OUT_DIRS_X86_64 + app_out_dir, with static guards tying the map to the gate scripts, the shell copy, and step_full's eight-app loop"
  - "x86_64/src/{syscall.rs,platform/q35/timer.rs}: the q35 millisecond-to-tick conversion for sleep/alarm (const-pinned), the fpprobe note-delivery root cause"
  - "witness/WITNESS-LOG.md + evidence/stop-c2/ + evidence/stop-c3/: the stop-C2/C3 verdicts, the TCG owner report, the green fpprobe and tap-venue results, the kernel hash proof, and trimmed redacted evidence"
  - "witness/equivalence.tsv: the recorded HARN-08 expectations (T1-OK 39, AUTH-OK 10, CATS-DONE 1, stress tap label) with per-row rationale, verified by stop C3 and approved 2026-11-23"
  - "harvey-flip/HARVEY-FLIP.md: APPROVED 2026-11-23; HARVEY-COMMIT c697666ba315f137ca2fd11768f835d04bd9f018; HARVEY-PUSHED none"
affects: [01-28, HARN-02, HARN-08]

actuals:
  tokens: 18886
  tasks: 4
  commits: 9

plan_head_before: 4f9697d1af8c9894898f7950d7a83cd78f074d57
plan_head_after: 66189ff7207b3f312448942c029eeb105e969ae5

tech-stack:
  added: []
  patterns:
    - "A q35 syscall argument in milliseconds is not a timer tick: the 100 Hz slice (TICK_MS=10) needs a ceiling conversion; the aarch64 twin's 1 kHz tick made the inherited `now + ms` tick-identical and hid the port's bug"
    - "A full-tier capture loop walks the full app list, never the whole app→dir map: the tap-venue rows belong to their own steps and a green GATE PASS must not exit 1 on their absent dirs"
    - "A tap app refuses slirp by contract, so the witness preflight exports R9_NET=tap explicitly for both doctor and app"
    - "An equivalence row whose Harvey baseline predates the current marker set carries a `recorded` expectation plus a per-row rationale; the gate's committed expect file remains the acceptance"
    - "The same serial signature (note-spin/note-regs 0 times) can be a missing IRQ boundary or a ten-times-late alarm; the code timeline and the venue's spin duration disambiguate it"
    - "A halted summary is a resume point: the C3 rerun proved the four red steps and the same summary closes as complete with only the approvals and the owner push outstanding"
    - "A prepared Harvey diff is applied only after a green witness set and the explicit APPROVE; stage the diff's exact 36 paths, never sys/** (the 11 APFS font artifacts), and verify the commit stat against the diff"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/summary.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/env.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/full.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/full.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/soak.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/stress.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/equiv/report.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/equiv/rows.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/store-verify.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/kernel.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/secret-scan.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/survivors.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/soak.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/equiv/report.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/equiv/rows.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/kernel.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/store-verify.log
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/summary.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/env.txt
    - .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/tcg-owner-report.txt
    - .planning/phases/01-stage-p0-cross-build-harness/01-34-SUMMARY.md
  modified:
    - .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
    - .planning/phases/01-stage-p0-cross-build-harness/witness/WITNESS-LOG.md
    - .planning/phases/01-stage-p0-cross-build-harness/witness/equivalence.tsv
    - .planning/phases/01-stage-p0-cross-build-harness/harvey-flip/HARVEY-FLIP.md
    - xtask/src/gate.rs
    - nix/gates/boot-r9-x64.sh
    - x86_64/src/syscall.rs
    - x86_64/src/platform/q35/timer.rs

key-decisions:
  - "fpprobe's stop-C failure is the q35 millisecond/tick conversion, not a missing interrupt-return note boundary: sys_alarm/sys_sleep inherited aarch64's `now + ms` while q35 ticks at 100 Hz, so alarm(50) armed at 500 ms and the two ring-3 spin rounds finished first on KVM; TCG (where the same rounds were green on 2026-11-22) outlasts 500 ms. The fix is a ceiling ms→ticks conversion with compile-time asserts and a host unit test; the trap.rs boundary is intact (0ffcec6/e9e5365). Stop C2 confirms it: `fpprobe: ok`, `T1-OK fpprobe`, 39/0."
  - "A green `GATE PASS full` must exit 0: step_full iterates the eight FULL_APPS only, because the soak/stress map rows have no capture in a full run and produced two FULL-NO-CAPTURE lines after the gate had passed."
  - "Both tap steps export R9_NET=tap: the soak app refuses slirp by contract; the stress app's implicit tap default is now explicit. The seam precondition remains a named failure."
  - "The four recorded equivalence expectations are verified by stop C3 (tier1 39/39, auth 10/10, churn 1/1, stress tap 1/1), all 10 rows match, and the owner APPROVED them 2026-11-23 as part of HARN-08."
  - "The new boot-iso outputHash sha256-b6OcAb2pOufpr5AqJj2SqhXnrTYsIQRTgkh9FcgtNT0= (owner commit 2bc5478) is proven by stop C3: KERNEL-BOOTISO-BUILD-OK + REBUILD-OK, kernel ELF 2377dc2f…"
  - "The standalone amd64 TCG gate is PASS by owner report (GATE-PASS, 2026-11-23); the GATE-PASS control-no-rio lines in the full/stress captures are the drawterm legs, not the standalone run."
  - "The owner approved the prepared Harvey diff and it was applied from base 25d97dc4d as Harvey c697666ba315f137ca2fd11768f835d04bd9f018 (36 paths, +285/−9972, zero font paths). The commit is unpushed; the owner push is the one remaining step."

requirements-completed: []

coverage:
  - id: D1
    description: "The soak/stress preflight reads the r9-built amd64 tree's netaddr seam and both apps run the tap venue explicitly"
    requirement: HARN-02
    verification:
      - kind: e2e
        ref: ".planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/soak.log (doctor PASS from the tree's seam; L2-TRUTH tap venue; ALL STAGES PASS)"
        status: pass
      - kind: e2e
        ref: ".planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/stress.log (tap venue label + ALL STAGES PASS)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The full runner and equivalence step use each app's actual R9_GATE_OUT short name, and boot-r9-x64 has a per-app log"
    requirement: HARN-08
    verification:
      - kind: unit
        ref: "xtask/src/gate.rs#gate::tests::full_tier_output_dirs_match_the_scripts_and_witness_map"
        status: pass
      - kind: unit
        ref: "xtask/src/gate.rs#gate::tests::full_capture_loop_covers_only_the_full_tier"
        status: pass
      - kind: e2e
        ref: ".planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/equiv/report.tsv (all 10 rows match; boot-r9-x64 1/1/1)"
        status: pass
    human_judgment: false
  - id: D3
    description: "fpprobe's note-delivery failure is fixed: the real handlers run and the tier-1 gate passes"
    requirement: HARN-08
    verification:
      - kind: unit
        ref: "x86_64/src/syscall.rs#sleep_and_alarm_convert_milliseconds_to_the_100hz_slice (host leg; compile-time asserts in every kernel build)"
        status: pass
      - kind: e2e
        ref: ".planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/full.log (fpprobe: ok, T1-OK fpprobe, 39 expected, 0 T1-FAIL)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The stop-C2 and stop-C3 verdicts and trimmed evidence are recorded"
    verification:
      - kind: other
        ref: "witness/WITNESS-LOG.md (six VERDICT stopc2-* + five VERDICT stopc3-* lines) + evidence/stop-c2/ + evidence/stop-c3/ (credential scan 0)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The equivalence expectations reflect the current r9/tap outcomes and are approved by the owner as part of HARN-08"
    requirement: HARN-08
    verification:
      - kind: e2e
        ref: ".planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/equiv/report.tsv (all 10 rows match, including the four recorded expectations)"
        status: pass
    human_judgment: true
    rationale: "The expected counts are an owner acceptance decision (HARN-08), not an automated assertion; the owner APPROVED them 2026-11-23 and the gate's committed expect files are unchanged."
  - id: D6
    description: "The new boot-iso outputHash is built and --rebuild-checked"
    verification:
      - kind: e2e
        ref: ".planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/kernel.log (KERNEL-BOOTISO-BUILD-OK + REBUILD-OK on b6Oc; ELF 2377dc2f…)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The standalone amd64 TCG gate is green"
    verification:
      - kind: manual_procedural
        ref: "owner report 2026-11-23: nix develop -c cargo xtask gate --arch x86-64 --accel tcg -> GATE-PASS (WITNESS-LOG VERDICT stopc3-tcg; evidence/stop-c3/tcg-owner-report.txt)"
        status: pass
    human_judgment: false
  - id: D8
    description: "The approved Harvey diff is applied and committed with the exact path set and no font path"
    verification:
      - kind: other
        ref: "Harvey c697666ba315f137ca2fd11768f835d04bd9f018: 36 files changed, +285/−9972; 0 sys/lib/(postscript|troff) or devpost.add paths; HARVEY-FLIP.md records the approval"
        status: pass
    human_judgment: false

# Metrics
duration: 54min
completed: 2026-11-23
---

# Phase 1 Plan 34: Stop-C2/C3 fixes — tap seam, capture paths, fpprobe, and the Harvey flip Summary

The x86_64 witness set is green end to end: stop C2's full tier 8/8 with
`fpprobe: ok` and tap stress, stop C3's soak/equivalence/kernel/store-verify,
and the owner's standalone TCG `GATE-PASS`. The four HARN-08 equivalence
expectations are approved, and the approved Harvey diff is applied and
committed at `c697666ba` (36 paths, +285/−9972, no font path) — unpushed;
the owner's push is the one remaining step.

## Accomplishments

1. **Tap preflight and venue.** `venue_gate` realises
   `.#harvey-tree-amd64` and runs the doctor with `HARVEY=<tree>` and
   `HARVEY_NETADDR_SEAM=<tree>/build/x86_64/root-tree/lib/netaddr.rc`;
   a missing seam is a named `seam-unavailable` / `seam-missing`
   precondition, never a venue verdict. Both the doctor and the app carry
   `R9_NET=tap`. Stop C3: `netip: L2-TRUTH tap venue`, ALL STAGES PASS.
2. **Capture paths.** The app→`R9_GATE_OUT` map covers the eight full
   apps plus the two tap rows; `step_full` copies only the eight, and
   `step_equiv` counts the same real dirs. `boot-r9-x64.sh` tees the
   terminal boot to `target/gates/boot-r9-x64/boot-r9.log`. Static tests
   pin the map to the gate scripts, the shell copy, and `step_full` to
   `FULL_APPS`. All 10 equivalence rows match.
3. **fpprobe.** Root cause: q35's 100 Hz slice (`TICK_MS = 10`) with
   aarch64-shaped `now + ms` armed `alarm(50)` at 500 ms; the two ring-3
   spin rounds finished first on the KVM host (TCG's slower spins always
   won the race, which is why the same rounds were green on 2026-11-22).
   Fixed with a ceiling ms→tick conversion for sleep/alarm and
   compile-time asserts; stop C2 printed `fpprobe: ok`, `T1-OK fpprobe`,
   39 expected T1-OK and zero T1-FAIL.
4. **Stop-C2/C3 record.** Eleven verdicts with trimmed, redacted evidence
   under `witness/evidence/stop-c2/` and `witness/evidence/stop-c3/`
   (credential scan 0): full PASS (8/8), soak FAIL→PASS, stress PASS,
   equiv FAIL→PASS (10/10), store_verify PASS twice, kernel
   PENDING→PASS (build + rebuild on `b6Oc`), TCG PASS (owner report).
5. **HARN-08 equivalence.** `equivalence.tsv` records T1-OK 39,
   AUTH-OK 10, CATS-DONE 1400 = 1 and the stress tap venue label, each
   with a rationale; the `recorded` source tells the comparator the count
   is an owner-approved expectation, not a stale capture. The owner
   approved the four on 2026-11-23; no gate or expect file changed.
6. **The Harvey flip.** The prepared diff (sha256 `f7b0272d…`) was
   applied from base `25d97dc4d` with `git apply` and an explicit 36-path
   `git add --`. The commit `c697666ba315f137ca2fd11768f835d04bd9f018`
   matches the prepared stat (36 files, +285/−9972), carries zero
   `sys/lib/(postscript|troff)` or `devpost.add` paths, and leaves the 11
   pre-modified font files unstaged. `HARVEY-FLIP.md` records
   `HARVEY-COMMIT`, `HARVEY-APPROVED: 2026-11-23 owner`, and
   `HARVEY-PUSHED: none`.

## Verdicts

| Step | Verdict | Evidence |
|------|---------|----------|
| full | PASS (8/8; `fpprobe: ok`, 39/0) | `evidence/stop-c2/full.tsv`, `full.log` |
| soak | PASS (tap L2-truth, 9 tcpblast, zero FAIL) | `evidence/stop-c3/soak.log` |
| stress | PASS (tap venue, 9 tcpblast, zero FAIL) | `evidence/stop-c2/stress.log` |
| equiv | PASS (all 10 rows match) | `evidence/stop-c3/equiv/{report,rows}.tsv` |
| store_verify | PASS (both trees; C2 and C3) | `evidence/stop-c3/store-verify.log` |
| kernel | PASS (BUILD + REBUILD OK on `b6Oc`) | `evidence/stop-c3/kernel.log` |
| TCG | PASS (owner report, standalone amd64) | `evidence/stop-c3/tcg-owner-report.txt` |

## Deviations from Plan

The owner's stop-C2 return exposed three harness defects and one stale
hash; the C3 rerun proved the fixes. Applied under the deviation rules:

1. **[Rule 1 — Bug] Full capture check counted the separate tap steps.**
   `GATE PASS full` still exited 1 with two `FULL-NO-CAPTURE` lines
   because `step_full` walked the whole app→dir map. Fixed by looping
   `FULL_APPS`; a static test pins it.
2. **[Rule 1 — Bug] Soak ran slirp.** `venue_gate` set `HARVEY`/the seam
   but not `R9_NET`; the soak app's contract refuses slirp. Both tap
   steps now export `R9_NET=tap`.
3. **[Rule 2 — Missing critical] Equivalence baselines were stale.**
   `T1-OK 39/36`, `AUTH-OK 10/5`, `CATS-DONE 1/2` and the tap run's
   absent slirp label were expectation drift, not gate failures. Recorded
   the stop-C2 expectations with per-row rationale; owner approval was the
   explicit gate (HARN-08) and C3 verified all four.
4. **[Rule 1 — Bug] The boot-iso hash was unverified.** The owner's
   commit `2bc5478` carried the new `outputHash`; the C2 `kernel` step was
   stale. Stop C3 proves `KERNEL-BOOTISO-BUILD-OK` + `REBUILD-OK`.

## Issues Encountered

- **The Harvey commit is unpushed.** `c697666ba` is one ahead of the
  owner's `master`; the branch is `ahead 2` with the pre-existing
  planning-freeze commit `25d97dc4d`. The owner push is the remaining
  action; the executor does not push Harvey.
- **x86_64 host unit test not run on the Mac.** `cargo xtask test`
  exercises the host arch (aarch64 here); the conversion is
  compile-time asserted in every kernel build, so `cargo xtask
  check`/`clippy` carry it. The owner's x86_64 host runs the test leg.

## Authentication Gates

None.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. The one approved-write boundary was
crossed exactly as planned: `git apply` + the explicit 36-path
`git add --` in `/Users/neilskinner/code/harvey`, no `sys/**`, no font
path, no push. No STATE.md/ROADMAP.md change.

## Next Phase Readiness

- **The witness set and every approval are complete.** Only the owner's
  Harvey push remains:

      git -C /Users/neilskinner/code/harvey push origin HEAD:master

  (the branch is the owner's current branch, `master`; the push publishes
  the pre-existing `25d97dc4d` freeze commit too).

- **01-28 was not started.** No pin bump and no further Harvey change.
- **After the push:** reply `Harvey pushed: <branch> <sha>`; the next
  continuation records `HARVEY-PUSHED` and proceeds to 01-28.

## Self-Check: PASSED

- `cargo test -p xtask gate::` 10/10 PASS; `cargo xtask clippy` and
  `cargo xtask check` exit 0.
- `witness/evidence/stop-c2/` has 12 files and `stop-c3/` 8; each
  ≤200 lines; `grep -acE 'auth-creds|[Pp]assword='` is 0 on every file.
- `WITNESS-LOG.md` has six `VERDICT stopc2-*` and five `VERDICT
  stopc3-*` lines (soak, equiv, kernel, store-verify, tcg).
- Commits through `66189ff` are ancestors of HEAD;
  `git rev-list --count 4f9697d..HEAD` = 9 before this records update.
- Harvey: HEAD `c697666ba315f137ca2fd11768f835d04bd9f018`, parent
  `25d97dc4d4976bfb7411f01970755acf6261c81b`, 36 files changed,
  +285/−9972, zero font paths; the 11 font modifications remain
  unstaged; `master` is ahead 2 and unpushed.
- STATE.md and ROADMAP.md were not modified (worktree mode).
