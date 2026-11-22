---
status: complete
phase: 06-full-tree-userspace-wave-0-libraries-core-tiers
source: [06-VERIFICATION.md]
started: 2026-11-05T17:30:00Z
updated: 2026-11-05T18:05:00Z
---

## Current Test

[testing complete]

## Tests

### 1. First hosted run of the rewritten phase6.yml on push: confirm wave0-host, tier-9k, tier-r9, and x86_64-poolpanic legs green on ubuntu-latest
expected: all four legs green; the tier-9k leg's first hosted observation is honestly PENDING FIRST PUSH (05-13/06-08 precedent — recorded in 06-EVIDENCE.md). Push and watch `gh run list --workflow phase6-userspace`. On red, the tier-9k job's step summary carries the failure digest.
result: pass
notes: User confirmed 2026-11-05 — all CI jobs green. (An earlier report of a tier-donor step-5 failure was traced to a stale pre-retirement run, not the current workflow.)

## Prior Cycle (2026-11-05, pre-gap-closure — kept for history)

### A. First hosted run of the original phase6.yml (with the 9front tier-donor leg)
result: issue — wave0-host red (hostbin not populated; 'nix run .#setup' precondition), x86_64-poolpanic red (pcc missing), tier-r9 green, tier-donor red at donor provisioning. Resolution: G-06-1 gap closure (06-09/06-10) — the tier-donor leg was RETIRED per maintainer direction 2026-11-05 and replaced by the self-provisioned tier-9k job; wave0-host/poolpanic legs self-provision setup+toolchain.
severity: blocker (closed)

### B. Host-class watch item: dir-coverage.txt venue line on linux-x86_64 hosted runner
result: pass — drift diff redded BY DESIGN with exactly three flips (cmd/go build row, jpg visitation count); re-baseline standing rule applied; venue line now discloses host-class visit-count jitter (06-10).

## Summary

total: 1
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

- gap_id: G-06-1
  truth: "phase6.yml hosted run: wave0-host / tier-r9 / x86_64-poolpanic green on ubuntu-latest; tier-donor leg's behavior explained (CR-01 precondition red accepted, or donor provisioning wired)"
  status: resolved
  resolved_by: 06-09-PLAN.md + 06-10-PLAN.md
  resolved_at: 2026-11-05
  reason: "Hosted run: tier-r9 green; wave0-host/poolpanic red (missing setup/toolchain provisioning); tier-donor red at host toolchain bootstrap (nix gcc hardening) then at the CR-01 donor-pin precondition"
  severity: blocker
  test: 1
  artifacts: [.github/workflows/phase6.yml, sys/src/cmd/cpp/BUILD, sys/src/cmd/go/mkfile, build/aarch64/gethunk-repro-6l.sh, sys/src/BUILD_SETUP_NAMESPACE, build/aarch64/coverage-sweep.sh, sys/src/9k/k10/k10t1, build/mm/build-9k10t1.sh, build/mm/boot-9k10t1-sweep.sh]
  missing: []
  resolution: |
    Closed per maintainer direction 2026-11-05 (Harvey's OWN kernel as the x86_64 CI venue, 9front local-aarch64-only): 06-09 built the k10t1 Harvey-kernel tier venue (local green: T1-VENUE-PASS ok=36 skip=22+2libtest fail=0, evidence/06-09-tier-9k-local.log); 06-10 wired it into phase6.yml as the self-provisioned tier-9k job, retired the 9front tier-donor leg, and added the wave0-host venue-pin tripwire. wave0-host/tier-r9/x86_64-poolpanic hosted-green at run 42089373924. CR-01 resolved by retirement (06-REVIEW-DISPOSITION.md).

## Deferred Follow-Ups

- test: 1
  idea: "Fix the legacy harvey.yml workflow to build via nix so it is green moving forward (pre-dates this work; red streak on master recorded since 2026-11-04T12:01Z — pre-existing/unrelated to Phase 6)"
  deferred_at: 2026-11-05
