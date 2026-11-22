---
phase: 17-r9-kernel-flush-out
plan: 03
subsystem: kernel
tags: [r9, port, waitset, max-waiters, virtio, used-ring, wr-05, wr-06, flake-pin, fod-rewire, host-pins]

requires:
  - phase: 08-r9-tcp-ip-path-devip-compatible-net
    provides: the devip/virtio shared-code fix sites the Phase-7 findings name
  - phase: 17-r9-kernel-flush-out
    provides: the closure pipeline proven on the CR-01 exemplar (17-01) + the pin-advance lineage at 97a5b7b (17-02)
provides:
  - WR-05 CLOSED — every kernel waiter set sized by one port-level MAX_WAITERS=512 const (the live proc-table value on both arches), with a count+report loud full arm
  - WR-06 CLOSED — next_used rejects a used-ring id for an unposted slot before the recovery clear (the V5 '(device fault)' family)
  - the 07 ledger at 11 open (WR-05/WR-06 flipped fixed with pin+SHA cites); the FLUSHOUT 07-WR-05/07-WR-06 rows filled
  - the FOD re-wire lineage advanced 97a5b7b -> 658001b (Harvey pin b02408cad, lock-only) with the two-derivation byte-identity proof in three-way agreement with the FOD's own got-value
affects: [17-04 (FLUSH-03's remaining WR-01..04 + WR-07/08 dispositions), 17-05..17-07, VERIFICATION (walks 17-FLUSHOUT.md)]

actuals:
  tokens: 12300   # chars/4 over the realized diff: r9 28959 + Harvey 6474 + this SUMMARY, /4
  tasks: 3
  commits: 2      # measured: git rev-list --count a41e4f0..a9b538bd9 (pin advance + ledger closure; this SUMMARY's docs commit follows)
plan_head_before: a41e4f0081d2fc6dd4b19d14e31da97e314a5daa
plan_head_after: a9b538bd9

tech-stack:
  added: []
  patterns: ["the loud fixed-capacity arm: a per-set dropped counter + a console report at the drop site (the soverflows shape) — observable by pure-struct pins AND live post-mortems, never a panic, safe in every context register runs (syscall/boot only; take stays print-free for IRQ context)", "the used-ring slot-liveness reject as the fourth member of next_used's V5 validation set (range/even/len/liveness) — a duplicate chain-head completion is a loud Err, never a second retire"]

key-files:
  created: []
  modified: [../r9/port/src/{lib,netif}.rs, ../r9/port/src/devip/{stack,conv,netlog,proto,ipifc}.rs, ../r9/port/src/virtio.rs, ../r9/port/src/{devether_tests,devip_tests,virtio_tests}.rs, ../r9/flake.nix, flake.lock, .planning/milestones/v2.0-phases/07-r9-pci-ecam-virtio-net-devether/07-REVIEW-DISPOSITION.md, .planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md]

key-decisions:
  - "The loud full arm's mechanism (the flagged assumption, executor-shaped): a per-set `dropped: u64` counter + a `crate::println!` report at the drop site — 'devether:/devip: waiter set full — registration for pid N dropped (total dropped M)'. register runs only in syscall/boot contexts (take, the IRQ-side op, stays print-free), the counter is observable by the pins via #[cfg(test)] `dropped()` accessors (the house pattern for pin-facing helpers), and the console report is live-observable in-kernel"
  - "MAX_WAITERS lives at the port crate ROOT (lib.rs) — the single definition point all six consumer files import; its comment names the two arch MAX_PROCS tables it must track and the dev_churn 2048 budget relationship (churn procs are not net waiters; a set that ever fills there now reports loudly)"
  - "GREEN commits carry the fix(...) type per the plan's own 'fix(17-WR-05)/fix(17-WR-06) message family' mandate — these are hardening bug fixes, not features; the RED pins land as their own test(...) commits (the repo's fix+pin precedent from 17-01/17-02)"

patterns-established:
  - "Waiter-capacity pins assert the boundary from both sides: N retained exactly + the N+1st drop COUNTED (the loud observable), with the unchanged-below-capacity contract guarded by idempotence pins that pass in RED"
  - "Device-fault negatives grow beside their siblings: the unposted-slot pin copies the rig/device_complete harness shape verbatim from the out-of-range/odd-id negatives — the V5 validation set is now closed under duplicate completion"

requirements-completed: []   # FLUSH-03 stays open — 17-04 owns the remaining WR-01..04 fixes + the WR-07/08 dispositions (shared-ID gate correctly blocked marking)

coverage:
  - id: D1
    description: "WR-05: the WaitSet resize to the real table (512) + the loud full arm, all three copies and twelve buffers"
    requirement: FLUSH-03
    verification:
      - kind: unit
        ref: "cargo test -p port --lib -- waitset = 9/9 (netif_waitset_/stack_waitset_/conv_waitset_ — 512-retained, 513th-loud incl. dropped()==1, idempotence guards with dropped()==0 teeth); battery 112/112 (baseline 102)"
        status: pass
      - kind: integration
        ref: "grep -rn 'u64; 16\\]' port/src/ = 0 (baseline 18); boot-r9-smoke-x64 exit 0 + boot-r9-net-smoke-x64 exit 0 (ALL STAGES PASS) at pin b02408cad — the resized sets ran live under the net gate's devip waiters"
        status: pass
    human_judgment: false
  - id: D2
    description: "WR-06: the used-ring unposted-slot liveness reject in next_used + its negative pin"
    requirement: FLUSH-03
    verification:
      - kind: unit
        ref: "cargo test -p port --lib -- used_ = 4/4 (used_drain_rejects_unposted_slot_ids with the exact '(device fault)' string + the existing out-of-range/odd-id/len negatives unchanged); battery 112/112"
        status: pass
      - kind: integration
        ref: "LIVENESS-CHECK-IN-PLACE grep (the reject string present; the clear preceded by the posted[slot] check) + boot-r9-net-smoke-x64 exit 0 — the virtio-net RX retire ran the check live under real device completions"
        status: pass
    human_judgment: false
  - id: D3
    description: "The discipline half: push-before-pin, FOD re-wire with the two-derivation proof, lock-only pin advance, gates re-green, both ledgers closed"
    verification:
      - kind: integration
        ref: "R9-CLEAN-PUSHED + PIN-COMMIT-CLEAN (r9 97a5b7b..658001b pushed, origin/main..HEAD=0; Harvey HEAD=flake.lock-only); FOD proof: kernel A plain + kernel B wr05wr06-proof-b override -> byte-identical ISOs (file sha256 8f3f6e2a…), NAR sha256-0KGcgz… in three-way agreement with the FOD's own got-value; nix build .#boot-iso + --rebuild green; LEDGER-TWO-CLOSED (07 open count exactly 11) + FLUSHOUT-WR-ROWS-PRESENT"
        status: pass
    human_judgment: false

# Metrics
duration: 18 min
completed: 2026-11-12
status: complete
---

# Phase 17 Plan 03: WR-05 WaitSet Resize + WR-06 Used-Id Liveness Summary

**The two self-contained Phase-7 hardening classes closed where the code is live: every kernel waiter set resized by one port-level MAX_WAITERS=512 const with a count+report loud full arm (zero `u64; 16]` literals remain), and next_used now rejects a used-ring id for an unposted slot before the clear — both RED-proven by pure-struct host pins, FOD re-wired with the two-derivation proof, pinned lock-only, both x64 gates ALL STAGES PASS, the 07 ledger at 11 open.**

## Performance

- **Duration:** 18 min
- **Started:** 2026-11-12T21:51:53Z
- **Completed:** 2026-11-12T22:09:59Z
- **Tasks:** 3/3
- **Files modified:** 15 (r9 12 + Harvey 3, excluding plan metadata)

## The Two Fixes

**WR-05 (r9 `aea604c` RED -> `2bc52a7` fix).** One port-level const — `pub const MAX_WAITERS: usize = 512` at the port crate root (lib.rs), the value both arch tables carry as `MAX_PROCS`; port cannot import the arch const, so the single definition point carries the comment naming the tables it must track (and the dev_churn 2048 budget relationship). It replaces the three WaitSet copies' arrays (netif.rs, devip/stack.rs, devip/conv.rs `WaitPids`), all three `take` signatures, and the twelve `[0u64; 16]` caller buffers (netlog.rs, proto.rs, netif.rs x3, stack.rs x4, ipifc.rs, conv.rs x2). The full arm is loud: a `dropped` counter + a console report at the drop site (the soverflows shape) — never a panic, never silence; `take` (the IRQ-side op) stays print-free. The stale `MAX_PROCS=16 / the full case is impossible` comment text corrected at all three copies (the remaining mentions are the fix's own history notes).

**WR-06 (r9 `3bd8625` RED -> `abe72e7` fix).** `next_used` gains the exact check from the finding before the unconditional slot-recovery clear: `if !self.posted[slot] { return Some(Err("virtio: used-ring id for an unposted slot (device fault)")); }` — the fourth member of the V5 validation set (range/even/len/liveness). A lying device double-completing a chain head previously passed every check and retired the slot twice (one buffer page pushed into the free stack twice — two live RX chains DMA into the same page).

## TDD Gate Compliance

Both fix tasks carried `tdd="true"` — full RED/GREEN cycles with classified evidence:

| Task | RED | Evidence | GREEN | REFACTOR |
|------|-----|----------|-------|----------|
| 1 (WR-05) | r9 `aea604c` | RED_EVIDENCE_OK (TAP conversion of the libtest JSON stream; raw JSON in `evidence/17-03-red-*.json`) | r9 `2bc52a7` | none (first-landing code) |
| 2 (WR-06) | r9 `3bd8625` | RED_EVIDENCE_OK (`evidence/17-03-wr06-red-*.json`) | r9 `abe72e7` | none (a 9-line check) |

- Task 1 RED: 6/9 pins failed on their planned assertions (`take` returned 16 of 512, `left: 16 right: 512`) at all three homes; the 3 idempotence guards passed (the unchanged contract). The RED commit carries the minimal `pub(crate)` visibility lifts so the crate-root homes can construct the structs (conv::WaitPids was already pub(crate)) — visibility only, no behavior.
- Task 2 RED: the new negative failed on its planned assertion (the duplicate id retired as Ok); the 3 existing `used_` negatives passed unchanged.
- GREEN commits carry `fix(...)` (not `feat(...)`) per the plan's own message-family mandate — hardening bug fixes.
- GREEN strengthened the 513th-loud pins with the `dropped()` observable (assert `dropped()==1` / `==0` teeth) — documented in both commit messages.

## The Pin-Advance Lineage (97a5b7b -> 658001b -> Harvey b02408cad)

1. r9 `aea604c` test(17-03): WR-05 pins, RED
2. r9 `2bc52a7` fix(17-03): WR-05 — MAX_WAITERS=512 + the loud full arm
3. r9 `3bd8625` test(17-03): WR-06 pin, RED
4. r9 `abe72e7` fix(17-03): WR-06 — the unposted-slot liveness reject
5. r9 `658001b` fix(17-03): **FOD re-wire** — the stale wiring measured `got: sha256-0KGcgz…`; two independent fresh-kernel builds (kernel A plain + kernel B under a never-before-used `wr05wr06-proof-b` version override) through the non-FOD `lib.bootIso` realization produced the byte-identical ISO: file sha256 `8f3f6e2aa172bd6ce4741dc80da036bb87d0ebbe97b8c32ec977fde683856bda`, NAR `sha256-0KGcgzXbFAhTXUohb4ENFfRNGei6IfiQnpy5brKdayI=` (`d0a19c8335db1408535d4a216f810d15f44d19e8ba21f8909e9cb96eb29d6b22`) — three-way agreement with the FOD's own got-value; `nix build .#boot-iso` + `--rebuild` green
6. r9 pushed `97a5b7b..658001b` BEFORE the Harvey advance — push-before-pin held (`origin/main..HEAD` = 0)
7. Harvey `b02408cad` chore(17-03): pin -> 658001b, **flake.lock only** (asserted PIN-COMMIT-CLEAN)
8. Harvey `a9b538bd9` docs(17-03): the 07-ledger rows + FLUSHOUT cells

## Gates Re-Green at the New Pin (the set run)

| Gate | Result | Evidence |
|------|--------|----------|
| `nix run .#boot-r9-smoke-x64` | exit 0 | ALL STAGES PASS — boot, echo (typed+answered), empty-line safety, ls/ps/cat sweep, DEL safety, restart, pid-warning-absent, nic-bind (`/tmp/17-03-smoke.log`) |
| `nix run .#boot-r9-net-smoke-x64` | exit 0 | ALL STAGES PASS — chain boot, hostfwd listeners observed host-side, ipconfig static+dhcp, DUPBIND-REJECTED, loopback ping+udp echo, NOADDR-REJECTED, tcp/7 echo, marker, pid-warning-absent, nic-bind (`/tmp/17-03-netsmoke.log`) |

The net gate is the live re-proof for both classes: devip's waiters (stack/conv/netlog/proto/ipifc) exercised the resized sets + the loud arm's non-interference, and the virtio-net RX retire ran the liveness check under real device completions. Host batteries: port 112/112 (baseline 102; the plan's 101 baseline was 17-02's churn_ladder +1), x86_64 bins 69/69.

## Task Commits (Harvey repo)

1. **Pin advance** - `b02408cad` (chore, lock-only: r9 97a5b7b -> 658001b)
2. **Ledger closure** - `a9b538bd9` (docs: 07-REVIEW-DISPOSITION WR-05/WR-06 -> fixed + the FLUSHOUT cells)

(r9's 5 commits listed in the lineage above; RED-evidence JSONs ride `evidence/17-03-*.json`.)

## Ledger Closure (D-13/D-16)

- **07-REVIEW-DISPOSITION.md**: WR-05 and WR-06 rows (YAML + table) flip `open` -> `fixed`, Source cells citing the r9 SHAs + pin test names; `open: 13` -> `11` (exactly these two; the IN rows stay untouched per Pitfall 10)
- **17-FLUSHOUT.md**: the `07-WR-05`/`07-WR-06` rows filled (action=fixed, pins, commits, ledger refs); the row-note corrected — 17-04 owns WR-01..04 + the WR-07/08 dispositions, 17-03 closed the two self-contained classes

## Decisions Made

- The loud-arm mechanism (flagged assumption): per-set `dropped` counter + console report; `#[cfg(test)]` accessors per the house pin-surface pattern (the initial un-gated accessors raised dead_code warnings in the kernel builds — gated before commit)
- Pin placement (flagged assumption): the existing crate-root homes (devether_tests for netif, devip_tests for stack+conv) with pub(crate) visibility lifts riding the RED commit — pure-struct pins, never the published-device path (Pitfall 8)
- The `-- waitset` filter gate ran as the name-filter variant (all nine pin names carry `waitset`): 9 passed / 0 failed

## Deviations from Plan

None - plan executed exactly as written. (The flagged assumptions resolved as above; the battery baseline was 102, not the plan's 101 — 17-02's churn_ladder pin grew it; the plan's gate "grow past 101" holds at 112.)

## Issues Encountered

None. The FOD re-wire needed one cycle (both fixes landed before the single re-wire, unlike 17-02's two-cycle plan); the classifier needed the evidence record path passed relative to the Harvey root (`path escapes its allowed directory` from the r9 cwd — rerun from the repo root).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 17-04 proceeds on the WR-05/WR-06-hardened kernel at pin `b02408cad` (r9 658001b): the cap-walk audit (WR-01..04) + the WR-07/08 D-15 dispositions complete FLUSH-03's ledger sweep
- The loud full arm is the standing guard for any future MAX_PROCS/MAX_WAITERS drift; the 07 ledger's remaining 11 rows (WR-01..04, WR-07/08, IN-01..05) are 17-04's and later phases' to close

## Self-Check: PASSED

- Files exist on disk: the 12 r9 sources + flake.nix (committed, tree clean), Harvey flake.lock + both ledgers (committed)
- Commits verified: r9 aea604c/2bc52a7/3bd8625/abe72e7/658001b all on origin/main (pushed, unpushed=0); Harvey b02408cad/a9b538bd9 ancestors of HEAD
- Plan-level verification re-run post-commit: u64-16 grep = 0; `-- waitset` = 9/9; `-- used_` = 4/4; battery 112/112; R9-CLEAN-PUSHED + PIN-COMMIT-CLEAN; smoke=0 + net=0 at the pin; LEDGER-TWO-CLOSED (11 open) + FLUSHOUT-WR-ROWS-PRESENT

---
*Phase: 17-r9-kernel-flush-out*
*Completed: 2026-11-12*
