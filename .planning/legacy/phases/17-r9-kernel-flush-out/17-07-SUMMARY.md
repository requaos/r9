---
phase: 17-r9-kernel-flush-out
plan: 07
subsystem: kernel-net
tags: [r9, devip, wake-path, race, triple-fault, stress-gate, kvm, root-cause, windows-ledger]

requires:
  - phase: 17-r9-kernel-flush-out (17-06)
    provides: the 17-05/06 pin base (r9 59d28fa) with all rc-corner sweeps green — the userspace state this plan's stress load rides
provides:
  - the committed CR-07 reproduction instrument (boot-r9-stress-x64: the reduced stress gate + the venue-aware netstress.rc fixture + the R9_STRESS_* shape hatches) that fires the class at will in ~25s
  - the first LIVE captures of the #66 class anywhere: 3 fires on the uninstrumented pinned kernel (2 pre-fix r9 59d28fa + 1 AT the fix pin r9 8f3ff11), all retained as evidence
  - 17-CR07-ROOTCAUSE.md — the investigation record: the window characterization, the structural exonerations of the three recorded candidates, the KVM capture ceiling established empirically, the 30/30 narrowing matrix localizing the class to the full 8-way concurrent bulk transfer, the refutation of the strongest-candidate fix, and the D-09 fold verdict (SPLIT)
  - the NET_WAITERS deregister counterpart (r9 d4a5d38): a real stale-registration defect closed with 4 deterministic pins (the sem machinery's register/deregister pairing brought to devip) — hardening BESIDE the open row, never its closure
  - WINDOWS #47 closed (discipline families) and #50 waived (3-part frozen-class split); #66 enriched and OPEN; the 08 ledger CR-07 row enriched and skipped; 17-FLUSHOUT.md COMPLETE (zero placeholder cells, 30 rows, the full pin-advance lineage + gate-re-green record)
affects: [Phase 18 (auth substrate — the open #66 class fires under many-waiter net load), Phase 19 (cpu/drawterm — aan over real net is the #47/#66 load family), any plan that re-opens the CR-07 hunt from this record]

actuals:
  tokens: 39300    # chars/4 over the realized diff (harvey 138517 + r9 18581 = 157098/4; the SUMMARY's own chars excluded); estimate was 75000 — the plan's cost went to wall-clock diagnosis (23+3+2+30 gate runs), not diff volume
  tasks: 3         # executed to their honest ends; Task 3's cure evidence gate failed (see Deviations)
  commits: 3       # MEASURED: git rev-list --count 9ca7adfca..HEAD (the docs/metadata commit follows)

# Plan-commit ledger (#3968) — measured
commits: 3
plan_head_before: 9ca7adfca34b4864a470f7d014783ffcd8dee7f8
plan_head_after: fdd7626c871c821b16f2299444b3727c23292c45
r9_commits: [d4a5d38, 8f3ff11]   # the class fix + the FOD re-wire, pushed (origin/main = 8f3ff11)

tech-stack:
  added: []        # no packages; qemu -d cpu_reset/int/guest_errors are qemu platform flags (the KVM capture ceiling is an empirically established platform fact)
  patterns: ["the memory-side reproduction loop: an uninstrumented-pinned-kernel stress app whose -d capture + serial signature + fast-fail arms catch a 1-in-10 race at will (the #66 class now reproducible in ~25s)", "honest-gate discipline: when the plan's own evidence rules refute the fix, the row stays open with the enriched record — a mechanical gate pass on a prose accident (the word 'fixed' in a not-cured row) is removed, not exploited"]

key-files:
  created:
    - build/x86_64/stress-r9.expect
    - build/x86_64/root-tree/lib/netstress.rc
    - nix/stress-r9-net-x64.sh
    - .planning/phases/17-r9-kernel-flush-out/17-CR07-ROOTCAUSE.md
    - .planning/phases/17-r9-kernel-flush-out/evidence/17-07-cr07-*   # 14 evidence files: both pre-fix fires, the fix-pin fire, the 23-run hunt log, the 10-run diag2 log, the n25 series, the narrowing matrix, the RED evidence
  modified:
    - flake.nix                # the boot-r9-stress-x64 app
    - flake.lock               # the pin advance 59d28fa -> 8f3ff11 (lock-only, own commit)
    - ../r9/port/src/devip/stack.rs      # deregister_net_waiter + WaitSet::remove + NetWaiterScope
    - ../r9/port/src/devip/proto.rs      # the scope at 9 blocking fns
    - ../r9/port/src/devip.rs            # the scope at listen_open + err_read
    - ../r9/port/src/devip_tests.rs      # the 4 new pins
    - ../r9/flake.nix                    # the FOD re-wire (two-derivation proof)
    - .planning/WINDOWS.md               # #47 fixed / #50 waived / #66 open-enriched
    - .planning/milestones/v2.0-phases/08-.../08-REVIEW-DISPOSITION.md  # CR-07 row enriched, stays skipped
    - .planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md            # COMPLETE + the lineage

key-decisions:
  - "17-07: the D-09 fold verdict is a SPLIT — #66 is the CR-07 class (reproduced live 3x, stays OPEN); #47 is the WEDGE form, closed-by-discipline (the landed re-probe/register-per-iteration families; its exact shape green at every x86_64 pin since 14-09); #50 is a different class entirely (aarch64 devpipe residence), waived with a 3-part disposition"
  - "17-07: the fix evidence failed — the N-series at the fix pin refuted the deregister counterpart as the CURE (the class fired at run 3, 25s, byte-consistent signature). Per D-03/T-17-14's own letter the row CANNOT read fixed; per honesty it cannot read waived either (a live ~8% crash class). The fix stands as correct hardening (real defect, 4 pins, all gates green); the row's enriched record + the committed reproduction instrument are the deliverables"
  - "17-07: N restated 10 -> 25 before the series (the measured reduced-shape rate 2/23 ~ 8.7% vs the assumed 20%: 0.913^N <= 0.11 needs N=25) — the plan's own restates-never-lowers rule; the series then failed at run 3, making the restatement moot but the failure decisive"
  - "17-07: the narrowing matrix (30/30 green: 8/4/2-way at depth 1 + 4/2/1-way at depth 512) localizes the class to the FULL 8-way concurrent bulk transfer — a per-event race scaling with event-rate x exposure, not a single-ingredient defect the axes could isolate"
  - "17-07: the KVM capture ceiling is now an empirical platform fact: -d int,guest_errors is BLIND at death under KVM in-kernel irqchip (empty even on the firing runs), -d cpu_reset logs only the post-reset register state, userspace-APIC is refused ('KVM does not support userspace APIC'), and TCG-tap wedges at the pre-ARP ping (the TCG full-cascade venue is unreachable on the class's own venue)"
  - "17-07: the series' run 1 rode the FULL default pipeline and runs 2+ rode R9_SKIP_BUILD=1 (the pinned kernel + run-1's just-rebuilt userland — one userland build for series homogeneity; the 15-08 tier-witness tree-history finding is itself a reason NOT to rebuild mid-series)"

patterns-established:
  - "the reduced-stress reproduction loop as a committed gate (presence markers, exact-count post-passes, the #66 RESET signature as a fast-fail arm, -D per-run cascade files) — the template for every future rare-race hunt in this repo"
  - "env hatches on committed gates for diagnosis (R9_QEMU_APPEND, R9_MACHINE, R9_STRESS_*) — host-side, guest-invisible, defaulting to the committed shape; the GATE always runs the defaults"

requirements-completed: []   # FLUSH-01 partial (CR-01..06 closed in 17-01; CR-07 investigated+hardened, NOT cured — stays open); FLUSH-04's FLUSHOUT table complete and walkable but the phase's ledgers did not all close (CR-07/#66 open). Neither requirement is honestly complete.

coverage:
  - id: D1
    description: "the CR-07 stress instrument (expect driver + venue-aware fixture + wrapper + flake app) — runs green on a healthy kernel, catches the class fast, venue-honest"
    requirement: FLUSH-01
    verification:
      - kind: other
        ref: "nix eval .#apps.x86_64-linux.boot-r9-stress-x64.program -> PROGRAM-EVAL-OK; probe nix run .#boot-r9-stress-x64 -> exit 0 + STRESS-APP-RUNS (ALL STAGES PASS, 9/9 tcpblast count, tap venue label asserted)"
        status: pass
    human_judgment: false
  - id: D2
    description: "the root-cause investigation: the live reproduction (3 fires), the candidate exonerations, the capture ceiling, the narrowing matrix, the fold verdict"
    requirement: FLUSH-01
    verification:
      - kind: other
        ref: "grep gates: ROOTCAUSE-RECORDED (candidate + fold verdict present); retained evidence 17-07-cr07-fire1/2 + fix-pin fire + hunt/diag2/n25/narrowing logs — all under evidence/"
        status: pass
    human_judgment: false
  - id: D3
    description: "the hardening fix + pins + class commit + push + FOD re-wire + pin advance + gates re-green (Task 2's full discipline chain)"
    requirement: FLUSH-01
    verification:
      - kind: unit
        ref: "cargo test -p port --lib = 124/124; -- wake filter = 6/6 (4 new pins, RED-proven compile-class: evidence/17-07-red-evidence.txt); cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu = 76/76; R9-CLEAN-PUSHED + PIN-COMMIT-CLEAN; FOD two-derivation proof (A d4a5d38 vs B 4be2f58 kernel-neutral clone: byte-identical ISO 2bd99a04...); smoke/net-smoke/churn gates ALL STAGES PASS at pin 8f3ff11"
        status: pass
    human_judgment: false
  - id: D4
    description: "the CURE — the N-series of consecutive green stress runs at the final pin (Task 3's centerpiece evidence)"
    requirement: FLUSH-01
    verification:
      - kind: other
        ref: "the N=25 series FAILED AT RUN 3 (2 greens, then the class fired at 25s at the fix pin — evidence/17-07-cr07-fix-pin-fire.serial.log): the fix is refuted as the cure; STRESS-N10-GREEN / STRESS-N25-GREEN NOT earned"
        status: fail
    human_judgment: false
    rationale: "automated and decisive: the failure IS the finding; recorded honestly rather than retried into a green streak"
  - id: D5
    description: "the fold closures + FLUSHOUT completion (Task 3's ledger work)"
    requirement: FLUSH-04
    verification:
      - kind: other
        ref: "#47 fixed + #50 waived (native resolve fields); #66 OPEN enriched (the honest verdict — ROW-66-UNRESOLVED fires by design); the 08 ledger CR-07 row stays skipped with the enriched backbone (the accidental 'fixed' prose match REMOVED so the gate fails honestly); FLUSHOUT-COMPLETE (0 placeholder cells, 31 '| ' lines, the full lineage)"
        status: pass
    human_judgment: false
    rationale: "the D5 verdict itself (which rows close vs stay open) follows the documented fold-split record; a human may wish to review the open-row call"
  - id: D6
    description: "the honest-open outcome for the centerpiece: WINDOWS #66 and 08-CR-07 stay open with the enriched record, the reproduction instrument, and the reopener conditions"
    requirement: FLUSH-01
    verification: []
    human_judgment: true
    rationale: "the decision to leave the phase's centerpiece row open (rather than a waiver or a weak fix claim) is a judgment call this record makes deliberately — the verifier/user should ratify it"

# Metrics
duration: 3h 44min
completed: 2026-11-13
status: halted
---

# Phase 17 Plan 07: CR-07 — the root-cause + fix + pins plan Summary

**The #66 class reproduced LIVE for the first time anywhere (3 fires on the uninstrumented pinned kernel, one of them AT the fix pin), the investigation exonerated all three recorded candidates structurally, localized the class to the full 8-way concurrent bulk transfer via a 30/30-green narrowing matrix, landed a real hardening fix (the NET_WAITERS deregister counterpart, 4 pins, full discipline chain) — and the stress series at the fix pin REFUTED that fix as the cure, so the row stays OPEN with the enriched record, the committed reproduction instrument, and honest gates.**

## Performance

- **Duration:** 3h 44min active (wall ~4h; the hunt/diag/narrowing loops ran unattended in the background)
- **Started:** 2026-11-13T02:52:14Z
- **Completed:** 2026-11-13T06:36Z (records) — the status is halted, not complete
- **Tasks:** 3 executed to their honest ends (Task 3's cure-evidence gate failed)
- **Files:** 23 changed in Harvey (+2 commits in r9)

## Accomplishments

- **The reproduction (the phase's reason to exist):** the #66 intermittent burst-leg triple fault — never before caught — fired 3× under the committed reduced-stress instrument, always ~25 s into the burst window, always the same signature (empty-status exit cascade → SeaBIOS reset chain → NO kernel trap dump). Evidence retained for every fire (serials, pcaps, the cpu_reset record, loop logs).
- **The investigation record** (17-CR07-ROOTCAUSE.md): the three recorded candidates exonerated STRUCTURALLY on x86_64 (the Sfmask=If cli invariant verified at its arming site — the IRQ-context lock walks cannot contend by construction); the KVM capture ceiling established empirically (four avenues, each closed loudly); the fold verdict SPLIT (#66 = the class; #47 = the wedge form, closed-by-discipline; #50 = a different class, waived frozen).
- **The hardening fix** (r9 d4a5d38): register_net_waiter had 11 call sites and no deregister — the stale-registration herd — closed with the sem machinery's own register/deregister pairing (NetWaiterScope + deregister_net_waiter + WaitSet::remove at all 11 blocking fns), RED-proven (compile-class E0425 ×4) and GREEN after (port 124/124, wake filter 6/6, x86_64 76/76), FOD re-wired with the two-derivation proof (A vs kernel-neutral clone B: byte-identical ISOs), pin advanced lock-only, smoke/net-smoke/churn gates ALL STAGES PASS at the new pin.
- **The narrowing matrix** (30/30 green): the launch storm alone (8 connects + dc churn + handshakes) never fires; ≤4-way full-depth transfer never fires; the class requires the FULL 8-way concurrent bulk transfer — the highest-density interleaving of RX-IRQ wake → schedule → devip drive → TX post → tick preempt the kernel sees anywhere.
- **The ledgers closed honestly:** #47 fixed, #50 waived, FLUSHOUT COMPLETE (zero placeholder cells + the full pin-advance lineage 208ceba → every fix SHA → every own-commit advance), and the two gates that would have passed dishonestly (#66's status grep on a prose accident; the 08 ledger's 'fixed' match) made to fail on purpose — matching the truth.

## Task Commits

Each task was committed atomically:

1. **Task 1: the stress instrument + the root-cause record** — `5a30511b7` (feat: expect driver + fixture + wrapper + flake app + the record + 9 evidence files)
2. **Task 2: the fix + pins + pin advance** — r9 `d4a5d38` (fix: the deregister counterpart + 4 pins) + r9 `8f3ff11` (fix: FOD re-wire, two-derivation proof) + Harvey `1c51844f2` (chore: pin advance, lock-only)
3. **Task 3: the honest verdict records** — `fdd7626c8` (docs: the record's post-fix section, WINDOWS/08-ledger/FLUSHOUT, the narrowing evidence, the shape hatches)

**Plan metadata:** this SUMMARY + STATE/ROADMAP (docs commit follows).

## Files Created/Modified

See frontmatter key-files. The evidence set (14 files) covers every fire and every loop.

## Decisions Made

See frontmatter key-decisions. The load-bearing one: **the fix evidence failed, so the row stays open** — the plan's own D-03/T-17-14 bar ("a fix proven only under instrumentation does not close the row") applies equally to a fix refuted by the stress series, and a waiver of a live ~8% crash class would be dishonest.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The venue-label echo lines broke rc parsing**
- **Found during:** Task 1 (the probe run)
- **Issue:** `echo netstress: venue tap (L2-truth venue, #66's own)` — unquoted parens are rc grouping tokens and `#` starts a comment: the fixture died at parse ("netstress.rc:133: syntax error")
- **Fix:** bare-ASCII marker lines (the 15-02 rc-dialect lessons applied); parse re-verified under plan9port rc
- **Files:** build/x86_64/root-tree/lib/netstress.rc
- **Verification:** the probe run reached the burst and passed all stages
- **Committed in:** 5a30511b7

**2. [Rule 3 - Blocking] Diagnosis hatches on the committed gate**
- **Found during:** Task 1 (the escalated-capture loop needed different qemu flags/machine opts than the committed shape)
- **Fix:** R9_QEMU_APPEND + R9_MACHINE (wrapper) and R9_STRESS_ROUNDS/BURSTN/BURSTROUNDS (driver) — host-side, env-defaulted, the GATE runs the committed defaults
- **Files:** nix/stress-r9-net-x64.sh, build/x86_64/stress-r9.expect
- **Verification:** the diag2 loop (cpu_reset escalation) caught fire 2 with the reset record; the narrowing matrix ran through the shape hatches
- **Committed in:** 5a30511b7 + fdd7626c8

**3. [Rule 3 - Blocking] The FLUSHOUT seed prose carried the word 'pending'**
- **Found during:** Task 3 (the zero-pending gate was unsatisfiable as seeded)
- **Fix:** the seed note reworded to past tense; the duplicated #47 seed row deduped (31 '| ' lines = header + 30, the gate's own minimum)
- **Files:** 17-FLUSHOUT.md
- **Verification:** FLUSHOUT-COMPLETE echoed
- **Committed in:** fdd7626c8

**4. [Rule 1 - Bug] The mechanical gate passed on a prose accident**
- **Found during:** Task 3 (the verify run)
- **Issue:** `grep -q '| CR-07 |.*fixed'` matched the enriched row's prose "cannot honestly flip to fixed" while the Disposition cell truthfully reads skipped — a false gate pass
- **Fix:** reworded ("flip its Disposition to cured") so the gate fails, matching the truth
- **Files:** 08-REVIEW-DISPOSITION.md
- **Verification:** the gate now fails; the row's Disposition cell = skipped
- **Committed in:** fdd7626c8

---

**Total deviations:** 4 auto-fixed (3 blocking, 1 bug) — plus the CENTRAL deviation below, which is not auto-fixable.

**THE CENTRAL DEVIATION (the halted status): Task 3's cure-evidence gate failed.** The N-series (restated to N=25 per the measured rate) failed at run 3 — the class fired at the fix pin with the byte-consistent signature. Under the plan's own rules this is NOT skippable noise: it is the refutation of the fix-as-cure. The consequences, executed honestly: WINDOWS #66 stays OPEN (enriched, not flipped); the 08 ledger CR-07 row stays skipped (enriched); the Task-3 verify gates STRESS-N10-GREEN and FOLDS-CLOSED(for #66) are NOT earned; requirements FLUSH-01 and FLUSH-04 are NOT marked complete. FLUSHOUT (the walkable index) completed with the honest open cells. **Impact:** the phase's centerpiece closes as investigated + hardened + reproducible-at-will rather than cured; the hunt resumes from this record when a venue exists that can reach the faulting context (the reopener conditions are in the record and both ledger rows).

## Issues Encountered

- The TCG full-cascade venue is unreachable on the class's own venue (the fixture wedges at the pre-ARP ping under TCG×tap; the TCG CI legs run slirp — a different L2 story). Recorded in the root-cause record §2; not retried on slirp (venue comparability with the 16-08 record).
- The first narrowing probe was mis-wired (an env the driver did not read); killed, the driver gained the shape hatches, and the matrix ran properly. No committed artifact carried the mistake.
- The pre-existing eqn.c working-tree modification and the untracked scratch files in the Harvey tree were left untouched (out of scope, untouched by every commit).
- The pre-existing #68 services dns-stage trap (17-04/17-06 records) remains out of this plan's authority — no flake.lock-adjacent services run was attempted beyond the standing A/B knowledge.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- The reproduction instrument is committed and proven (fires the class at will in ~25 s); the narrowing matrix's shape hatches make load-space sweeps cheap for whoever re-opens the hunt.
- The reopener conditions (record §7): a host-side venue that reaches the faulting context (a qemu with KVM userspace-APIC support, or a TCG-reachable L2 path for the tap fixture), or instrumentation that does not heal the window (the 16-08 observer-effect record is the bar).
- Phase 18 (auth substrate) proceeds on the x86_64 gates' green state at pin 8f3ff11 (smoke/net-smoke/churn ALL STAGES PASS; the #66 class fires only under the 8-way bulk-transfer load, which the auth tier does not generate).

---
*Phase: 17-r9-kernel-flush-out — Plan 07*
*Completed (halted): 2026-11-13*

## Self-Check: PASSED

All key-files.created exist on disk; the three task commits (5a30511b7, 1c51844f2, fdd7626c8) are ancestors of HEAD; the r9 commits (d4a5d38, 8f3ff11) are pushed to origin/main; the wake-filter pins re-verified green at close (6/6).
