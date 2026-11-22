---
phase: 16-host-net-modes-networking-exit-gate
plan: 07
subsystem: docs
tags: [hnet-03, hnet-04, net-modes, tap, slirp, vmnet, nixos, socket-vmnet, evidence-consolidation, drift-notes, d-04, d-08, d-10, d-14]

requires:
  - phase: 16-host-net-modes-networking-exit-gate
    provides: "all seven sibling plans' landed surfaces (16-01's emitter/TOML data, 16-02's tap_ seam, 16-03's doctor remediation text, 16-04's --net surface + pin, 16-05's soak machinery, 16-06's re-keyed gate + CI job, 16-08's green-twice verdict captures) — this plan is the phase's documentation + evidence consolidation half"
provides:
  - "build/x86_64/README.md extended with the Phase-16 net-modes companion: the three modes + one-command surface, the D-04 slirp remap rationale with the 1:1 opt-in variant, the D-08 tap topology with the NOT-TESTED real-LAN paragraph, the TESTED NixOS page leading with the two live-probed divergences (nosuid helper; allow virbr0 trap) + the trustedInterfaces doctor-gap warning, the DOCUMENTED-NOT-TESTED generic-Linux page, the DOCUMENTED-MANUAL darwin socket_vmnet page (A4-flagged argv + fd-passing alternative), the five-surface contracts table, and the probe-evidence notes"
  - "16-EVIDENCE.md — the phase's consolidated evidence record: the NETIP-08 verdict statement (tap captures as sole accepted evidence, r9 SHAs named), the pin-advance lineage with the own-commit rule, green-twice declarations per gate, the 7-file retained-artifacts index (existence-checked line by line), requirement->proof mapping, the A1-A7 assumption outcomes, both PENDING FIRST PUSH items, and the venue-honesty recap"
affects: [Phase 18/19 (the auth/drawterm legs cite the D-04 remap + the hostfwd table), any future kernel-flush-out plan (the evidence index names the open burst-triple-fault entry), future darwin bring-up (the DOCUMENTED-MANUAL page is the starting point)]

actuals:
  tokens: 8947    # chars/4 over git diff 2a7c3a9b..0f252fb6f (estimate was 30000 — the docs consolidate records the phase already wrote)
  tasks: 2
  commits: 2      # MEASURED: git rev-list --count 2a7c3a9b0245f786e0fd7e30565423a6fbe692ab..HEAD
plan_head_before: 2a7c3a9b0245f786e0fd7e30565423a6fbe692ab
plan_head_after: 0f252fb6f965f0310a6300dfddcf9d8f4e310a1f

tech-stack:
  added: []        # plain markdown, no imports, no packages (T-16-SC: the trivial surface noted)
  patterns:
    - "venue honesty in prose matching the asserted labels in the gates: every documented-only leg (generic-Linux, darwin, real-LAN) carries an explicit not-tested label IN ITS SECTION HEADER or lead sentence — a README reader cannot mistake documented for tested"
    - "tested/documented boundary spelled as an honest-boundary note: the NixOS page states exactly which remediation lines the green-twice runs exercised (bridge/addr/sysctl/ACL/firewall) and which they did not (networking.nat masquerade rendering — WARN class, never dialed past the default route)"

key-files:
  created:
    - .planning/phases/16-host-net-modes-networking-exit-gate/16-EVIDENCE.md
  modified:
    - build/x86_64/README.md

key-decisions:
  - "The README EXTENDS the existing Phase-13 companion in place rather than being created fresh: build/x86_64/README.md already existed as the x86_64 drift-notes home (the boot-chain/accel/timer/drift sections the phase's own docs reference) — the net-modes sections join it as sibling per-topic contracts, exactly the aarch64 README's multi-topic discipline"
  - "The trustedInterfaces firewall opening is documented as MANDATORY tap-remediation checklist material with the measured evidence (4x SYN, zero SYN-ACK without it) — honoring the 16-08 deferred item's explicit instruction that the README must name every line the venue needs even where the doctor's WARN-class read cannot surface it"
  - "The evidence file separates RETAINED captures (the 7 on-disk files, indexed with existence cross-checked) from RECORDS THAT LIVE ELSEWHERE (doctor live outputs and negative-control logs quoted in the sibling SUMMARYs, the failed-run signature in deferred-items) — the acceptance criterion's every-indexed-file-exists holds literally while the plan's broader index duties are still met"
  - "The precision note in 16-EVIDENCE.md: the retained netsmoke/netsoak captures are pty-side (guest markers only); the hostfwd-listening stage-ok lines and driver-console spawn echoes are recorded in the SUMMARYs; the 16-04 boot capture uniquely carries the host-side argv echo — so the r9-pin-in-evidence claim (r9-netdev-args-d1c4849 in the echoed argv) is anchored to the one capture that actually shows it"

patterns-established:
  - "Evidence consolidation names the SHA a capture RODE in the index table itself (one column), so the pin lineage and the artifact index cannot drift apart"
  - "Assumption-ledger close-out verdicts keep the five-valued vocabulary (CONFIRMED / FALSIFIED / SPLIT / NOT PROVEN / STILL UNOBSERVED) — A3 and A4 staying open is recorded as a finding, not papered over"

requirements-completed: [HNET-03, HNET-04]

coverage:
  - id: D1
    description: "build/x86_64/README.md — the net-modes companion doc with all nine planned sections, every documented-only leg labeled, the two live-probed NixOS divergences verbatim, and the five-surface contracts table"
    requirement: HNET-03
    verification:
      - kind: command
        ref: "Task 1 verify battery: test -s FILE-OK; not-tested/manual label count 4 (>=3: real-LAN x2, generic-Linux, darwin); D-04/D-08/D-10 + 'allow br0' + socket_vmnet + 5670 anchors present; seam literal count 7 (>0)"
        status: pass
      - kind: command
        ref: "Task 1 acceptance: all five surfaces present (boot-r9-x64, net-doctor, boot-r9-net-soak-x64, host-echo, r9-netdev-args); DIVERGENCES-OK (nosuid + 'access denied by acl file' + virbr0); labels at README lines 186/188/276/295 — generic-Linux and darwin and real-LAN each carry one"
        status: pass
    human_judgment: false
  - id: D2
    description: "16-EVIDENCE.md — the consolidated phase evidence record a reviewer audits NETIP-08 against: verdict statement, pin lineage, green-twice declarations, artifact index, assumption outcomes, PENDING FIRST PUSH, venue honesty"
    requirement: NETIP-08
    verification:
      - kind: command
        ref: "Task 2 verify battery: test -s FILE-OK; soak-capture reference lines 4 (>=2); 'PENDING FIRST PUSH' + 'r9 SHA' anchors present; evidence dir lists 7 files"
        status: pass
      - kind: command
        ref: "Task 2 acceptance cross-check: every indexed evidence/ file exists on disk (all 7 FOUND line by line; the single regex artifact was the a/b.shorthand, both real .log files FOUND); NETIP-08 verdict names both tap captures + d1c4849dfa1d769240630c29b9b03b88d5819558; marker claims re-verified on the captures at consolidation time (done=9/9, fail=0/0, l2=1/1, netipdone, arp; netsmoke notl2=2/2)"
        status: pass
    human_judgment: false

duration: 19min
completed: 2026-11-11
status: complete
---

# Phase 16 Plan 07: Net-Modes README + 16-EVIDENCE.md Consolidation Summary

**HNET-03/HNET-04's documented halves landed as the net-modes companion in build/x86_64/README.md — NixOS TESTED (leading with the nosuid-helper and allow-virbr0 trap, plus the measured trustedInterfaces lesson), generic-Linux and darwin and real-LAN each explicitly not-tested — and 16-EVIDENCE.md consolidates the whole phase: the NETIP-08 green-twice verdict naming the tap captures and their r9 SHAs, the pin lineage, the A1-A7 assumption outcomes, and both PENDING FIRST PUSH items.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-11-11T20:24:15Z
- **Completed:** 2026-11-11T20:44:06Z
- **Tasks:** 2/2
- **Files modified:** 2 (1 created, 1 extended)

## Accomplishments

- `build/x86_64/README.md` gained the full Phase-16 net-modes companion (293 added lines in the aarch64 drift-notes discipline): the three modes + one-command surface with the fail-loud rules and the parallel-boot constraints; the slirp default's hostfwd table with the D-04 remap rationale (the live EPERM probe, `ip_unprivileged_port_start=1024`) and the 1:1 host-567 sysctl variant; the D-08 tap topology table; the TESTED NixOS page; the generic-Linux page (DOCUMENTED, NOT TESTED); the darwin socket_vmnet page (DOCUMENTED-MANUAL, the A4-flagged emitted argv + the fd-passing alternative + the entitlement wall); the five-surface scripts-and-contracts table; and the probe-evidence notes (ports matrix, device nodes, helper binaries, the r9 pin relationship with the 16-04 capture's `r9-netdev-args-d1c4849` anchor)
- The NixOS page leads with the two live-probed divergences verbatim (the nosuid store helper default on both qemu 11.1.2 builds — `helper=` MUST be explicit; the bridge.conf `allow virbr0`-vs-`br0` trap with the live `access denied by acl file` denial), carries the colmena note, the full declarative snippet (same content the doctor prints — canonical home vs live surface), AND the trustedInterfaces doctor-gap warning with the measured 4x-SYN/zero-SYN-ACK evidence — the 16-08 deferred item's explicit ask for this README
- `16-EVIDENCE.md` consolidates the phase per the 15-EVIDENCE precedent: the NETIP-08 verdict statement (the tap captures as sole accepted evidence, both runs at r9 `d1c4849dfa1d769240630c29b9b03b88d5819558`, the soak's slirp refusal quoted); the pin-advance lineage `af52fee -> d1c4849` with the own-commit rule at each hop and the 16-08 no-fix-SHA verdict (r9 exonerated, FOD clauses vacuous); green-twice declarations per gate with the pty-vs-driver-console precision note; the 7-file retained-artifacts index (every entry existence-checked line by line, each with its r9 SHA); requirement-to-proof mapping for HNET-01..05 + NETIP-08; the A1-A7 assumption outcomes (A1/A2/A5/A7 confirmed, A6 split, A3 still unobserved, A4 not proven — both honestly open); both PENDING FIRST PUSH items; the venue-honesty recap; and the open items carried past the phase

## Task Commits

Each task was committed atomically:

1. **Task 1: the build/x86_64 net-modes README** — `e79a587e4` (docs)
2. **Task 2: 16-EVIDENCE.md consolidation** — `0f252fb6f` (docs)

**Plan metadata:** this SUMMARY + STATE/ROADMAP/REQUIREMENTS updates.

## Files Created/Modified

- `build/x86_64/README.md` — the net-modes companion sections (modified: intro re-scoped, net-modes block after the accel contract, the composed-gates pointer, 430 lines total)
- `.planning/phases/16-host-net-modes-networking-exit-gate/16-EVIDENCE.md` — the phase evidence record (created, 112 lines)

## Decisions Made

See key-decisions in the frontmatter. The load-bearing ones:

1. **Extend, don't recreate** — the README existed as the Phase-13 companion; the net-modes sections join it as sibling per-topic contracts (the aarch64 multi-topic discipline), preserving content the phase's own docs reference.
2. **The firewall opening is checklist material** — documented with its measured failure signature, not just as a NixOS snippet line, because the doctor's WARN-class read demonstrably cannot surface it (the 16-08 wedge).
3. **Retained vs record** — the evidence index names only on-disk captures; doctor outputs and negative-control logs are cited by their durable record location (the sibling SUMMARYs), keeping the every-indexed-file-exists acceptance literal.
4. **Honest-boundary prose** — "TESTED" claims exactly what the green-twice runs exercised; `networking.nat` masquerade rendering is named unproven (WARN class, nothing dials past the default route).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking/plan-shape] The plan says "Create build/x86_64/README.md" but the file already exists**
- **Found during:** Task 1 (read_first: the aarch64 README precedent + the existing file)
- **Issue:** `build/x86_64/README.md` has been the x86_64 drift-notes home since Phase 13 (boot-chain, accel, timer, qemu-drift sections) — a literal create-from-scratch would have destroyed committed contract documentation the phase's own records reference
- **Fix:** the net-modes companion EXTENDS the file in place (intro re-scoped to name the Phase-16 sections; the nine planned sections inserted as siblings of the accel contract; the existing sections untouched) — the plan's must-haves truths are all content assertions, every one satisfied by the extension
- **Files modified:** build/x86_64/README.md
- **Verification:** the full Task 1 verify battery + acceptance criteria green on the extended file
- **Committed in:** e79a587e4

---

**Total deviations:** 1 auto-fixed (1 Rule 3 plan-shape)
**Impact on plan:** none on substance — every required section, label, anchor, and table landed; the only adaptation is where they live (extended file vs new file)

## Issues Encountered

None. One verify-mechanics note: the Task 2 acceptance cross-check's first extraction pass reported one shorthand artifact (`16-06-netsmoke-a/b.log` parsed as a stem without `.log`) — both real files were FOUND in the same pass; no evidence file is missing.

## User Setup Required

None - no external service configuration required. (Applying the NixOS tap remediation on any NEW host is the human's step — this README + the doctor are the two surfaces that print it; the primary host is already remediated.)

## Next Phase Readiness

- Phase 16 is fully landed: all 8 plans complete, every requirement's machinery + docs + evidence halves delivered; the phase's HNET-03/HNET-04 checkboxes may now complete (this was the last declarer of both shared IDs)
- Phases 18/19 inherit the documented remap surface (`127.0.0.1:5670 -> 567`, `127.0.0.1:17010 -> 17010`) and the README page their drawterm/auth docs cite
- The two PENDING FIRST PUSH hosted observations remain the push-time items (named in 16-EVIDENCE.md); the open burst-triple-fault finding (WINDOWS #66) is the natural first target of a Phase-17 kernel-flush-out plan

## Self-Check: PASSED

- Files: build/x86_64/README.md (extended, 430 lines), 16-EVIDENCE.md (created, 112 lines) both present (FOUND)
- Commits: e79a587e4, 0f252fb6f both ancestors of HEAD; commits measured from the plan ledger = 2 (no uncommitted code changes)

---
*Phase: 16-host-net-modes-networking-exit-gate*
*Completed: 2026-11-11*
