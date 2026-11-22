---
phase: 16-host-net-modes-networking-exit-gate
plan: 08
subsystem: net
tags: [netip-08, hnet-03, soak-gate, tap, l2-truth, green-twice, expect-driver, stage-budget, triple-fault-open]

requires:
  - phase: 16-host-net-modes-networking-exit-gate
    provides: "16-05's armed soak gate (peer + app + driver), 16-03's net-doctor, 16-04's emitter + the d1c4849 pin, 16-02's tap_ seam + netip-tap.rc fixture, and the user's host firewall remediation (trustedInterfaces br0, applied via the pre-plan checkpoint)"
provides:
  - "the NETIP-08 verdict leg: two consecutive green tap-leg soak captures + pcaps under the established evidence names (16-05-netsoak-a/-b.log/.pcap.gz) with byte-exact accounting (67108864 = 2048x32768 baseline + 8x 16777216 = 512x32768 burst, both directions, both runs)"
  - "the burst-wait stage-budget fix (soak_stage_timeout 2700, stage's optional per-call override) — the 600s TCG-carried window killed every first run MID-BASELINE"
  - "the first-run root-cause record: instrumented debugcon autopsy (port 0x402) exonerated the r9 kernel — the driver's own timeout kill masqueraded as a kernel wedge (torn serial lines, unacked finals, dead timers)"
  - "the open r9 finding: an intermittent burst-leg guest triple fault (1-in-5 runs, observer-healed) recorded with the diagnostic toolkit + the recommended stress-loop shape"
affects: [16-07 (the evidence index greps exactly these captures + names), any future r9 kernel-flush-out plan (the stress-loop recommendation)]

actuals:
  tokens: 4800     # chars/4 over the authored diff (driver fix + deferred-items + SUMMARY); estimate was 55000 — the plan's budget went to diagnosis, not code
  tasks: 2
  commits: 2       # MEASURED: git rev-list --count 1fb430010..HEAD
plan_head_before: 1fb430010c7d152ecf191f874b55b1b98aa7bc2
plan_head_after: c2b78b7bf47ae523ee7669b6c2c16c1c4d64e2ac

tech-stack:
  added: []        # no new packages; the qemu isa-debugcon (port 0x402) used for diagnosis is a qemu platform fact, not a dependency
  patterns:
    - "per-stage timeout overrides in expect drivers: a gate whose legs span a many-minute soak cannot inherit the sibling gates' TCG-sized 600s windows — the stage that bridges the soak (burst-wait) carries its own budget while every other stage stays tight (a stall stays loud everywhere)"
    - "the timeout-kill masquerade: a driver's own stage-timeout SIGKILL produces every symptom of a kernel wedge (mid-line serial truncation, unacked final segments, dead timers, silence) — an instrumented channel the kill cannot touch (debugcon on a separate chardev) is the discriminator"
    - "observer-healed races: a defect that never reproduces with any per-event trace attached cannot be root-caused by instrumentation — the honest output is a recorded finding + a purpose-built stress loop (rounds=1/burstn=8), never a blind fix"
    - "evidence pcaps gzip-compressed with md5 round-trip proof: 450MB of patterned payload -> 16MB, byte-faithful (gunzip md5 == raw md5), names carried as .pcap.gz beside the logs"

key-files:
  created:
    - .planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-05-netsoak-a.log
    - .planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-05-netsoak-b.log
    - .planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-05-netsoak-a.pcap.gz
    - .planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-05-netsoak-b.pcap.gz
  modified:
    - build/x86_64/netsoak-r9.expect
    - .planning/phases/16-host-net-modes-networking-exit-gate/deferred-items.md

key-decisions:
  - "The first-run failures were the GATE's, not the kernel's: the carried 600s per-stage budget cannot span the KVM-paced baseline (2048 rounds ~ 810s measured at ~2.5 rounds/s); every first run died mid-baseline to the driver's own timeout kill. Fixed by a burst-wait-only 2700s budget (the stage proc gained an optional per-call override)."
  - "r9 exonerated and NOT touched: an instrumented kernel (debugcon syscall/sched/IRQ/drive traces) proved the guest healthy at kill time (clean swtch resume markers, no drive-loop spin, no trap, round lines streaming). flake.lock stays at d1c4849 — no fix, no FOD re-wire, no pin advance (the acceptance criteria's push-before-pin and FOD clauses are vacuously satisfied)."
  - "The intermittent burst-leg triple fault is recorded OPEN, not blind-fixed: 1 failure in 5 full runs, never reproducing under any observer (per-syscall debugcon heals it; -d class observers perturb the same window). The fix-everything doctrine's bar (root-cause -> battery -> push -> FOD -> pin) cannot be met blind; the finding + toolkit + stress-loop shape live in the phase deferred-items and WINDOWS #66."
  - "Evidence pcaps ride the repo as .pcap.gz (md5 round-trip proven against the raw captures); only GREEN runs earn the evidence names — the failed first evidence attempt's copies were deleted."

patterns-established:
  - "Gate-driver budgets must be derived from the gate's own committed data (rounds x measured pace), never inherited from sibling gates' stage windows — and the derivation note belongs beside the constant."
  - "The pcap arithmetic triple-check (app round-lines x payload vs ISN-to-final-position vs the peer's byte accounting) catches silent data-path corruption that memcmp-through-an-echo cannot (an echo venue makes TX/RX confusion invisible to the byte-compare) — one run showed a 4172-byte phantom-read anomaly that never recurred and is part of the open finding's record."

requirements-completed: [NETIP-08, HNET-03]

coverage:
  - id: D1
    description: "The burst-wait stage-budget fix — the gate's own first-run killer, root-caused and closed"
    requirement: NETIP-08
    verification:
      - kind: command
        ref: "Full gate from scratch post-fix, uninstrumented pinned kernel: nix run .#boot-r9-net-soak-x64 (R9_NET=tap) -> ALL STAGES PASS, exit 0 (t1c run: baseline + burst 8x512 + stats + count post-pass 9/9)"
        status: pass
      - kind: command
        ref: "Task 1 marker assert on the terminal green capture: grep -o 'tcpblast: done' | wc -l == 9; arp view ok / L2-TRUTH tap venue / netip: NETIP-DONE present; zero netip: FAIL (grep -a — pty control bytes make the capture read as binary)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The green-twice evidence pair under the established names + pcaps, both runs riding the pinned r9 d1c4849"
    requirement: NETIP-08
    verification:
      - kind: command
        ref: "Back-to-back runs A+B both ALL STAGES PASS exit 0; evidence asserts: done=9/9, fail=0/0, arp=1/1, venue=1/1, netipdone=1/1 in both 16-05-netsoak-a.log and -b.log; peer accounting byte-exact both runs (67108864 + 8x16777216 both directions)"
        status: pass
      - kind: command
        ref: "r9 lineage asserted: git -C ../r9 status clean at d1c4849, unpushed count 0; harvey flake.lock r9 rev d1c4849 (unchanged through the plan)"
        status: pass
    human_judgment: false

duration: 274min
completed: 2026-11-11
status: complete
---

# Phase 16 Plan 08: NETIP-08 Verdict — Tap-Leg Soak Runs, Fix Loop, Green-Twice Summary

**The tap-leg verdict landed GREEN TWICE consecutively with byte-exact accounting on both runs — after the fix loop's decisive twist: the first-run "kernel wedge" was root-caused (instrumented debugcon autopsy) as the gate's own 600s stage budget killing the guest MID-BASELINE (fixed: a burst-wait-only 2700s budget), leaving r9 exonerated and the pin untouched at d1c4849; the one genuine kernel finding — an intermittent, observer-healed burst-leg triple fault (1-in-5 runs) — is recorded open with the full diagnostic toolkit and stress-loop shape.**

## Performance

- **Duration:** 274 min (4h 34m)
- **Started:** 2026-11-11T15:30:51Z
- **Completed:** 2026-11-11T20:04:55Z
- **Tasks:** 2/2
- **Files modified:** 6 (4 evidence artifacts created, driver + deferred-items modified)

## Accomplishments
- The NETIP-08 verdict leg: two consecutive green tap-leg soaks (baseline 2048 + burst 8x512) retained as `evidence/16-05-netsoak-a.log` / `-b.log` with pcaps (`.pcap.gz`, md5 round-trip proven) — every connection byte-exact both directions both runs, zero `netip: FAIL`, ARP L2-truth + venue labels + NETIP-DONE present in both
- The root-cause of the first-run failures: three full runs died at ~599s with torn serial output, unacked final segments, and total guest silence — a qemu `-d cpu_reset` + debugcon (port 0x402) instrumented kernel (syscall R+/R-/W+/W-, sched/pick/resume, IRQ+/-, wake, drive_stack entry/exit + spin detector — built, exercised live, then fully reverted) proved the guest healthy at kill: the expect driver's per-stage 600s timeout fired at the `burst-wait` stage while the baseline (~810s at the measured 2.5 rounds/s) was still streaming
- The fix: `soak_stage_timeout 2700` riding only `burst-wait` (the stage proc's optional per-call override); every other stage keeps the tight carried default
- r9 left byte-identical at d1c4849 (tree clean, pushed); flake.lock never advanced — no kernel fix, no FOD re-wire (both acceptance clauses vacuous)
- The one genuine defect surfaced — an intermittent burst-leg guest triple fault (empty-status exit cascade then CPU reset, no kernel dump) — recorded as an OPEN item (deferred-items + WINDOWS #66) with the reproduction statistics (n=5, 1 failure), the observer-healing evidence, and the recommended stress harness (rounds=1/burstn=8 loop on the uninstrumented pinned kernel with `-d int,guest_errors`)

## Task Commits

Each task was committed atomically:

1. **Task 1: the first soak run + the kernel-fix loop** — `0a29da7d9` (fix — the loop's terminal fix was harvey-side: the driver's stage budget; ../r9 exonerated, untouched)
2. **Task 2: green-twice — the evidence pair + consolidation** — `c2b78b7bf` (test — evidence logs + pcaps + the deferred-items record)

**Plan metadata:** this SUMMARY + STATE/ROADMAP/REQUIREMENTS updates.

## Files Created/Modified
- `build/x86_64/netsoak-r9.expect` — soak_stage_timeout 2700 + the stage override (the first-run killer closed)
- `evidence/16-05-netsoak-a.log` / `-b.log` — the green-twice pair (the plan's declared names)
- `evidence/16-05-netsoak-a.pcap.gz` / `-b.pcap.gz` — the runs' pcaps, gzip (450MB→16MB each, md5 round-trip proven)
- `deferred-items.md` — the burst triple-fault open item (+ the prior continuation's doctor-gap entry, committed here)

## Decisions Made

See key-decisions in the frontmatter. The load-bearing ones:

1. **The fix loop's terminal condition was met harvey-side.** The plan's loop ("diagnose -> fix in ../r9 -> push -> FOD -> pin -> re-run") assumed the surfacing defect would be an r9 kernel defect; the measured truth was a gate-data defect (the stage budget vs the leg's wall-clock). The fix landed where the defect lived.
2. **No blind fix for the triple fault.** The doctrine demands root-cause; the defect refuses observation (two instrumented runs green, three of four uninstrumented green, one reset). Recording it with a purpose-built reproduction path IS the fix-loop-honest outcome at this budget; a speculative patch would be unprovable and unreviewable.
3. **The pcap evidence naming.** The plan names `.pcap`; the retained files carry `.pcap.gz` (byte-faithful, proof recorded) — 900MB of raw patterned payload would have been repo-hostile; 16-07's consumer greps the LOG names and counts dir entries (both unaffected).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The plan's run commands cannot execute as written**
- **Found during:** Task 1 (the first run)
- **Issue:** the plan's literal `nix run .#boot-r9-net-soak-x64` runs with R9_NET unset -> the script's own default is slirp -> the app's venue-honesty refusal aborts every run before the doctor
- **Fix:** every run used `R9_NET=tap nix run .#boot-r9-net-soak-x64` (the plan's own `--net=tap` contract form)
- **Files modified:** none (invocation mechanics)
- **Verification:** six full runs executed end-to-end
- **Committed in:** n/a

**2. [Rule 3 - Blocking] The evidence-capture greps need binary-safe flags on this host**
- **Found during:** Task 2 (the asserts)
- **Issue:** the retained captures contain pty control bytes; GNU grep treats the files as binary and refuses to print counts (the plan's literal assert shapes fail confusingly)
- **Fix:** the asserts ran with `grep -a`; counts identical (9/9/0/1/1/1 per file)
- **Files modified:** none (verify mechanics)
- **Verification:** the pair-assert table in the run log
- **Committed in:** n/a

**3. [Rule 1 - Bug] The evidence-pair inline consolidation step lost the pcaps once**
- **Found during:** Task 2 (run A's consolidation)
- **Issue:** the console-scrape for the run's evidence-scratch dir returned empty inside the backgrounded one-liner (quoting); the pcap copy silently no-opped
- **Fix:** the pcaps were copied explicitly from the named scratch dirs (`/tmp/netsoak-r9-x64.DZD5Pe`, `.FiMDWY`) and verified by md5 before compression
- **Files modified:** none (mechanics)
- **Verification:** md5 round-trip of both .gz against both raw pcaps
- **Committed in:** n/a

---

**Total deviations:** 3 auto-fixed (3 Rule 3/1, all invocation/verify mechanics — zero plan-content deviations)
**Impact on plan:** none structural — every declared artifact, marker, count, and acceptance criterion landed as written

## Issues Encountered

**The intermittent burst-leg guest triple fault (OPEN — the plan's expected defect class, genuinely surfaced, root cause not isolatable in-phase).** Evidence-run A's first attempt: baseline 2048/2048 green, ~2 burst rounds in, the guest CPU-reset (terminal escape + SeaBIOS + GRUB on the capture; no kernel trap dump — the fault class that bypasses the IDT). Statistics across the plan: 5 full runs, 1 failure; both debugcon-instrumented runs green (the per-event trace closes the window); the failed run's pcap showed all six burst connections closed at non-round byte counts (the proc-exit fd closes) before the reset. The run-1-era phantom-read anomaly (app reads exceeding the wire by 4172 bytes, content-matching — one early run only) never recurred and is part of the finding's record. Full entry: `deferred-items.md` (burst triple fault) + WINDOWS #66. The failed run's captures were NOT retained as evidence (only green runs earn the names) — the signature is recorded in prose.

**Auth gates:** none. **The user's one manual step** (the host firewall `trustedInterfaces = [ "br0" ]` remediation) was completed before this executor started (verified live: doctor zero-FAILs, and the runs' SYNs answered).

## User Setup Required

None — the tap venue carried the pre-plan checkpoint's remediation; the doctor's masquerade WARN persists by design (never gates).

## Next Phase Readiness
- NETIP-08's verdict evidence is complete and greppable under the established names; 16-07's index consumes `16-05-netsoak-a.log` / `-b.log` verbatim (the `.pcap.gz` siblings keep the dir count and the run's wire truth)
- HNET-03's tested-on-primary-host claim is backed by these captures (KVM + br0 + the real host ARP answer, both runs)
- The open r9 finding (burst triple fault, observer-healed) is the natural first target of any Phase-17 kernel-flush-out plan — the stress-loop shape and the debugcon instrumentation set are documented and were exercised live this plan
- The parallel-run constraint stands: one soak at a time (the peer's blastport bind + one br0 leg)

## Self-Check: PASSED

- Files: evidence/16-05-netsoak-a.log, -b.log, -a.pcap.gz, -b.pcap.gz present (FOUND); netsoak-r9.expect carries soak_stage_timeout + the override (FOUND)
- Commits: 0a29da7d9, c2b78b7bf both ancestors of HEAD; commits measured from the plan ledger = 2 (no uncommitted code changes)
- Marker asserts re-run on both evidence files at SUMMARY time: done=9/9, fail=0/0, arp/venue/NETIP-DONE present in both

---
*Phase: 16-host-net-modes-networking-exit-gate*
*Completed: 2026-11-11*
