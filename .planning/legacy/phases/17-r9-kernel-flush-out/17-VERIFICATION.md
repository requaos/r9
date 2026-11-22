---
phase: 17-r9-kernel-flush-out
verified: 2026-11-13T15:18:29Z
status: passed
score: 6/6 must-haves verified
covered_files: [".planning/WINDOWS.md", ".planning/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/08-REVIEW-DISPOSITION.md", ".planning/phases/17-r9-kernel-flush-out/17-01-PLAN.md", ".planning/phases/17-r9-kernel-flush-out/17-01-SUMMARY.md", ".planning/phases/17-r9-kernel-flush-out/17-02-PLAN.md", ".planning/phases/17-r9-kernel-flush-out/17-02-SUMMARY.md", ".planning/phases/17-r9-kernel-flush-out/17-03-PLAN.md", ".planning/phases/17-r9-kernel-flush-out/17-03-SUMMARY.md", ".planning/phases/17-r9-kernel-flush-out/17-04-PLAN.md", ".planning/phases/17-r9-kernel-flush-out/17-04-SUMMARY.md", ".planning/phases/17-r9-kernel-flush-out/17-05-PLAN.md", ".planning/phases/17-r9-kernel-flush-out/17-05-SUMMARY.md", ".planning/phases/17-r9-kernel-flush-out/17-06-PLAN.md", ".planning/phases/17-r9-kernel-flush-out/17-06-SUMMARY.md", ".planning/phases/17-r9-kernel-flush-out/17-07-PLAN.md", ".planning/phases/17-r9-kernel-flush-out/17-07-SUMMARY.md", ".planning/phases/17-r9-kernel-flush-out/17-08-PLAN.md", ".planning/phases/17-r9-kernel-flush-out/17-08-SUMMARY.md", ".planning/phases/17-r9-kernel-flush-out/17-CR07-ROOTCAUSE.md", ".planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-rc-preterminus.sha", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-probe.log", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run01.serial.log", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run02.serial.log", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run03.serial.log", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run04.serial.log", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run05.serial.log", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run06.serial.log", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run07.serial.log", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run08.serial.log", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run09.serial.log", ".planning/phases/17-r9-kernel-flush-out/evidence/17-08-tcg-slirp-run10.serial.log"]
covered_digest: "v4:sha256:9d7c533ab440485f54a41f4f586c23d6d7adb393fb0d8b225824927c45a920c7"
behavior_unverified: 0
overrides_applied: 1
overrides:
  - must_have: "SC1/FLUSH-01 — the Phase-8 CR class 'multi-waiter RX-wake' (CR-07 / WINDOWS #66) is closed with tests or explicitly dispositioned with rationale"
    reason: "Maintainer-ratified explicit disposition at the 17-08 blocking-human checkpoint (Neil Skinner, 2026-11-13T14:42:08Z): investigated-to-the-capture-ceiling (the KVM ceiling of 17-CR07-ROOTCAUSE section 2, completed by section 8's TCG x slirp avenue — NO-FIRE-IN-SERIES) + hardened (r9 d4a5d38) + reproducible-at-will (boot-r9-stress-x64, ~25s) + the 3-part D-15 rationale with a venue-conditional reopener per section 7 — deferred/waived, NOT cured; FLUSH-04 ratified yes in the same decision (the verifier-confirmed letter: #44/#45 closed, every landed fix pinned)"
    accepted_by: "Neil Skinner"
    accepted_at: "2026-11-13T14:42:08Z"
re_verification:
  previous_status: gaps_found
  previous_score: 5/6
  gaps_closed:
    - "SC1/FLUSH-01 — the CR-07 / WINDOWS #66 class neither closed nor explicitly dispositioned: now explicitly dispositioned with rationale (maintainer-ratified waived/deferred, 3-part D-15) after the TCG x slirp last-avenue probe (17-08), with the reopener conditions and the reproduction instrument surviving intact"
  gaps_remaining: []
  regressions: []
---

# Phase 17: r9 Kernel Flush-Out Verification Report

**Phase Goal:** The r9 kernel is functionally hardened under real userspace load — the open Phase-7/8 review backlog (the CR classes, the RFMEM OOM leaks, the WR hardening gaps) plus the known kernel windows are closed with tests or explicitly dispositioned with rationale, every fix landing with a pin (host test or boot-test gate) per the established discipline. **Kernel work in sibling repo `../r9`.**
**Verified:** 2026-11-13T15:18:29Z
**Status:** passed
**Re-verification:** Yes — after gap closure (17-08, the CR-07/#66 terminus plan)

## Goal Achievement

Verified against the actual repos: Harvey at HEAD 35703c594 and the sibling r9 repo — clean tree, HEAD = origin/main = 8f3ff11fd12802da1a48c9fffcc6467d4a7954c8, exactly matching Harvey's flake.lock r9 pin. The gap-closure commits (e20ecccea, 0eed1d432, af7a00610, dbd357e3e, 35703c594) all exist; their diff is records-only (`.planning/` files exclusively — no kernel bytes, no flake change, no source change). Both r9 host batteries were re-run LIVE this verification at the unchanged pin, not trusted from SUMMARYs.

### The Gap Closure (17-08) — what changed since the 2026-11-13T07:11:20Z report

The single blocking gap — CR-07 / WINDOWS #66 neither closed nor explicitly dispositioned — closed via the goal's own "explicitly dispositioned with rationale" arm, through the exact route the prior report's override invitation prescribed:

1. **The last-avenue probe ran and is recorded.** `17-CR07-ROOTCAUSE.md` section 8: N=10 serialized TCG x slirp runs of the committed boot-r9-stress-x64 instrument (`R9_ACCEL=tcg R9_NET=slirp`), exactly one outcome token `Outcome: NO-FIRE-IN-SERIES` (line 90, the only `Outcome:` line in the section), the RACE-STRESS/DIAGNOSIS venue label, N stated with rationale, and the weak-negative semantics spelled out in the record itself (avenue exhaustion, never absence; P(0 fires in 10 | ~8-10%/run KVM rate) ≈ 35-43%). Evidence independently inspected this pass: `evidence/17-08-tcg-slirp-probe.log` carries 10 distinct `[run N]` lines (timestamps 14:19:06Z-14:29:11Z, serialized ~64-94s apart, unique scratch dirs); the 10 per-run serial captures exist, are all DISTINCT by sha256 (genuine separate runs), and each carries `netstress: STRESS-DONE` (1), `netstress: RACE-STRESS slirp venue not L2-truth` (1), exactly one SeaBIOS banner (the initial boot), and zero reset chains (the only "reset" match per capture is the TCP stats counter `EstabResets: 0`). The ~143-146 MB per-run TCG cascades were deleted per the host-hygiene rule with sizes recorded in the loop log's `[post-corrected]` lines.
2. **The maintainer terminus is recorded BEFORE any closure edit.** Section 9 carries the verbatim literal lines `Terminus: ratify-disposition` / `Decided-by: Neil Skinner` / `Decided-at: 2026-11-13T14:42:08Z` / `FLUSH-04: yes` (each exactly once in the file). The record through section 8 is hash-intact against its pre-terminus capture — recomputed this pass: `evidence/17-08-rc-preterminus.sha` (12e7eba5...) == sha256 of the file minus section 9. RC-EVIDENCE-BASE-INTACT holds.
3. **All five record surfaces carry one consistent ratified terminus.** WINDOWS #66: `status: "waived"`, reason = the 3-part D-15 rationale (WHERE / WHY-not-fixed / Reopener) with the ratification cite, `resolved_at: 2026-11-13T14:42:08Z` stamped, description untouched; ledger counts moved exactly 50/4 -> 49/5 (only #66 flips — the git diff confirms the row was the only WINDOWS change). 08-ledger CR-07: Disposition = the exact token `deferred` with the 17-08 ratification cite; the stale "carries the open row" parenthetical rewritten to the waived state ("NEVER a cure" stated on the row). 17-FLUSHOUT.md: exactly 41 table lines (40 -> 41, one records-only 17-08 lineage row), both the 08-CR-07 and #66 action cells read `dispositioned (deferred, maintainer-ratified 17-08) ... NOT cured`, all three pre-ratify literals gone (0 `OPEN, ENRICHED`, 0 `HARDENED, NOT CURED`, 0 `FLUSH-01 is therefore PARTIAL`). 17-VERIFICATION.md (the prior report): the invited overrides block only — the file minus that block recomputes to the plan's baseline sha e0747290... byte-identical (VERIF-INTACT verified this pass). REQUIREMENTS.md: FLUSH-01 and FLUSH-04 `- [x]`, both traceability rows `Complete` — exactly the ratified branch.
4. **The prior report's secondary finding closed:** the 08-ledger WR-03 Source cell now cites 17-02's both-tiers-green-twice discharge; `has NOT run` appears 0 times in the ledger.
5. **Records-only invariant held:** r9 clean at 8f3ff11 == origin/main through every step; flake.lock untouched (rev 8f3ff11fd12802da...); the closure diff touches `.planning/` exclusively.

The prohibitions of the gap-closure contract were checked against the records, not trusted: no cure claim anywhere (waived/deferred/NOT-cured vocabulary on every surface); the reopener conditions and the instrument survive intact (section 7 hash-preserved inside the intact sections 1-8; `boot-r9-stress-x64` still wired at flake.nix:353/:410 with all three instrument files present and R9_STRESS hatches in both wrapper and driver); the probe kernel was the committed uninstrumented pin (clean tree == committed bytes, no guest-visible instrumentation — the only captures are host-side serial + `-d` files); N=10 stated with rationale in the record; the venue label rides in every retained capture; D-12 row confinement confirmed by the diffs (WINDOWS: only #66; 08 ledger: only CR-07 and WR-03); the REQUIREMENTS checkboxes moved only as the recorded FLUSH-04: yes answer directs.

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1/FLUSH-01: the Phase-8 CR classes are closed with tests or explicitly dispositioned — conv_close FIN-drop (CR-01), permanent-send-error wedge (CR-02), semacquire wakeup loss (CR-03), ipifc multicast panic (CR-04), unbind stale-handle aliasing (CR-05), tcp_gc aliasing (CR-06), multi-waiter RX-wake (CR-07) | ✓ PASSED (override) | CR-01..06: closed with tests — ledger rows `fixed` with x86_64-keyed re-cites; batteries re-run green live THIS pass at the same pin (port 124/124, x86_64 bins 76/76; sem_ pins among them). CR-07/#66: now explicitly dispositioned with rationale — the goal's own accepted arm: WINDOWS #66 waived with the 3-part D-15 rationale + resolved_at; 08-ledger CR-07 `deferred` with the same rationale; sections 8-9 record the completed last-avenue probe (NO-FIRE-IN-SERIES, weak-negative semantics stated) and the verbatim maintainer ratification. Override: maintainer-ratified explicit disposition — accepted by Neil Skinner on 2026-11-13T14:42:08Z (the carried-forward overrides block matches section 9's Decided-by/Decided-at verbatim; the acceptance is what a waiver of a live ~8%/run class requires — the class is NOT claimed gone, the reopener and instrument stand) |
| 2 | SC2/FLUSH-02: the RFMEM OOM-path page leaks (the 6 WR findings) are closed with refcount proofs — leak gates demonstrate bounded memory under fork/exec churn | ✓ VERIFIED | Regression: r9 HEAD unchanged at 8f3ff11 (the exact commit live-verified 07:11:20Z with the audit table, the 4 ports in 3e8bcec, the tiered gates green twice, churn capture CATS-DONE 1400, WINDOWS #44 fixed+resolved_at) — the git object is immutable, nothing regressed; full port battery re-run live this pass 124/124. REQUIREMENTS FLUSH-02 checked + Complete |
| 3 | SC3/FLUSH-03: the Phase-7 WR hardening gaps are closed — cap-walk loud-abort holes, WaitSet-16-vs-proc-table sizing, used-id slot-liveness | ✓ VERIFIED | Regression: same immutable pin 8f3ff11 carries MAX_WAITERS=512 (lib.rs:130), the virtio rejects (:140/:170/:780), the q35 predicates — all live-verified 07:11:20Z (waitset 9/9, used_ 4/4, caps 5/5, battery 76/76); batteries re-run green this pass (124/124 + 76/76). 07-ledger state unchanged (WR-01..06 fixed, 07/08 deferred 3-part). REQUIREMENTS FLUSH-03 checked + Complete |
| 4 | SC4/FLUSH-04 (letter): the known kernel windows are closed or gated — fork+exec arena drain (#44), namec union-create miss (#45) — every flush-out fix lands with a pin | ✓ VERIFIED | #44 fixed (truth 2); #45 fixed (walk_one_union :341 + create_chan_union :460 + sys_create routing, r9 cfe7af0+421d5e1) — live-verified at this pin 07:11:20Z, namec pins 6/6; batteries green again this pass. The maintainer ratified the FLUSH-04 checkbox (FLUSH-04: yes, section 9) — REQUIREMENTS now checked + Complete, resolving the prior escalation item 2 |
| 5 | Goal discipline clause: every fix landing with a pin (host test or boot-test gate) | ✓ VERIFIED | All 30 FLUSHOUT rows carry pins + commit SHAs (re-walked this pass: 41 table lines, zero `pending` cells); the 17-08 round added NO fixes (records-only — its lineage row correctly reads "no r9 commits, no pin mechanics"); all 25+1 r9 commits previously verified exist and are pushed (HEAD == origin/main); Harvey pin-advance commit 1c51844f2 flake.lock-only |
| 6 | Goal index clause: the closure index (17-FLUSHOUT.md) is complete and VERIFICATION-walkable (D-16) | ✓ VERIFIED | Exactly 41 `\| `-leading lines (header + 30 rows + lineage header + 9 lineage rows incl. the one 17-08 row); zero pending/placeholder cells; the CR-07/#66 rows now carry the ratified disposition state (consistent with WINDOWS and the 08 ledger — the three no longer diverge); pin-advance lineage table intact |

**Score:** 6/6 truths verified (0 present, behavior-unverified; 1 passed via the carried-forward maintainer override)

### Advisory (New Scope, Unevidenced)

None. No new-scope Step 7 findings were raised this pass (the only markers found are pre-existing future-phase roadmap state — see Anti-Patterns).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.planning/phases/17-r9-kernel-flush-out/17-CR07-ROOTCAUSE.md` | sections 8 (probe) + 9 (terminus) | ✓ VERIFIED | Both present exactly once; one outcome token; verbatim decision lines; head hash-intact vs `evidence/17-08-rc-preterminus.sha` (recomputed) |
| `evidence/17-08-tcg-slirp-probe.log` + 10 per-run serials + baseline sha | the retained probe series | ✓ VERIFIED | 10 `[run N]` lines; 10 distinct non-empty serials (~105 KB each, distinct sha256s); baseline sha 65 B, recomputation matches |
| `.planning/WINDOWS.md` | #66 waived with 3-part rationale; counts 49/5 | ✓ VERIFIED | status/reason/resolved_at only fields changed (diff-confirmed); open=49 waived=5 fixed=14 |
| `08-REVIEW-DISPOSITION.md` | CR-07 `deferred` + cite; WR-03 re-cited | ✓ VERIFIED | Exact token in Disposition cell; `has NOT run` gone; only CR-07 + WR-03 rows changed (diff-confirmed) |
| `17-FLUSHOUT.md` | 41 lines, ratified cells, lineage row, zero stale literals | ✓ VERIFIED | All four conditions re-checked this pass |
| `.planning/REQUIREMENTS.md` | FLUSH-01..04 checked + Complete | ✓ VERIFIED | All four `[x]`, all four rows Complete |
| The 15 kernel/build artifacts of truths 2-5 (proc.rs OOM ports + sem pins, churngate.rs + config_q35churn.toml, churn/stress expect+sh+rc+c, lib.rs MAX_WAITERS, virtio.rs + virtio_tests.rs, pci.rs predicates, namec.rs + namec_tests.rs, devip/stack.rs deregister, rc/{simple,plan9,exec}.c) | unchanged at the pin | ✓ VERIFIED (regression) | r9 clean at 8f3ff11 == origin/main == flake.lock rev — byte-identical to the 07:11:20Z live-verified state; batteries re-run green |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| the committed instrument (boot-r9-stress-x64 + hatches) | the TCG x slirp series -> section 8's outcome token -> the checkpoint's evidence base | flake app + env hatches + the record | ✓ WIRED | App wired at flake.nix:353/:410; wrapper/driver/fixture files present with R9_STRESS hatches; the series' 10 live invocations are the retained evidence; section 8 cites them |
| the checkpoint's ratification | WINDOWS #66 + 08-ledger CR-07 + FLUSHOUT cells + VERIFICATION overrides + REQUIREMENTS checkboxes | section 9's Terminus token | ✓ WIRED | One decision, five surfaces, all consistent — verified surface-by-surface this pass; the override's accepted_by/accepted_at == section 9's Decided-by/Decided-at verbatim |
| 17-02's green-twice gate record | the WR-03 Source re-cite | the ledger row | ✓ WIRED | WR-03 cites the 17-02 both-tiers discharge; stale warning prose gone |
| (carried forward) port battery -> composed gates; MAX_WAITERS -> WaitSets; used-id check -> recovery clear; heap_stats -> churn bound; Caps::parse -> q35 bind; chain_snapshot -> union twins; #66 toolkit -> record -> fix -> pins; pg.lock sites -> #18 | as verified 07:11:20Z | — | ✓ WIRED (regression) | All ride the unchanged pin 8f3ff11; batteries re-run green live this pass |

### Data-Flow Trace (Level 4)

Kernel/records phase — no rendered-UI data. The prior pass's gate-data flows are unchanged at the immutable pin. This round's data: the probe's evidence chain (11 live instrument invocations -> per-run serials -> loop log -> section 8's outcome token) was traced to real captures with distinct checksums and consistent per-run markers — no fabricated or duplicated evidence.

### Behavioral Spot-Checks

Run live this verification in /home/req/code/r9 (nix develop, pinned nightly-2026-01-30):

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full r9 port host battery | `cargo test -p port --lib` | 124 passed / 0 failed | ✓ PASS |
| Full r9 x86_64 bins battery | `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` | 76 passed / 0 failed | ✓ PASS |
| CR-07 wake/deregister pins | `cargo test -p port --lib -- wake` | 6 passed / 0 failed | ✓ PASS |
| Probe evidence integrity | sha256 + marker scan of the 10 serials + hash recompute of record/baseline/report | all distinct, all green-classified, RC-EVIDENCE-BASE-INTACT + VERIF-INTACT hold | ✓ PASS |
| Repo/pin integrity | git: r9 HEAD == origin/main == 8f3ff11, clean; flake.lock rev == 8f3ff11fd...; closure commits exist; diff records-only | all hold | ✓ PASS |

### Probe Execution

| Probe | Command | Result | Status |
|-------|---------|--------|--------|
| The 17-08 probe series (already executed by the plan, evidence retained) | Inspected in place: 10 serialized runs + 1 verify re-run, all green, NO-FIRE-IN-SERIES | evidence verified this pass (not re-executed — the series is a diagnostic record, not a gate; re-running would produce a new series, not verify the retained one) | ✓ VERIFIED (log-corroborated + content-inspected) |
| r9 host batteries (the phase's pin venue) | the two `cargo test` runs above | 124/124 + 76/76 | PASS |
| Boot gates (smoke/net-smoke/tier1/churn/stress) | qemu-boot composed gates | Not re-run (minutes each; unchanged committed machinery + retained captures + live host pins at the unchanged pin — same disposition as the 07:11:20Z pass) | ? SKIP (log-corroborated) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| FLUSH-01 | 17-01, 17-07, 17-08 | Phase-8 CR classes closed with tests or explicitly dispositioned | ✓ SATISFIED | CR-01..06 fixed with live-green pins; CR-07/#66 explicitly dispositioned (maintainer-ratified D-15 waiver/defer — the goal's own disposition arm); REQUIREMENTS `[x]` + Complete |
| FLUSH-02 | 17-01, 17-02 | RFMEM OOM leaks closed with refcount proofs | ✓ SATISFIED | Verified 07:11:20Z at the same immutable pin; batteries green again this pass; checked + Complete |
| FLUSH-03 | 17-03, 17-04 | Phase-7 WR hardening gaps closed | ✓ SATISFIED | Same; checked + Complete |
| FLUSH-04 | 17-02, 17-05, 17-06, 17-07, 17-08 | Known windows #44/#45 closed or gated; every fix pinned | ✓ SATISFIED | Letter verified 07:11:20Z; checkbox ratified by the maintainer (FLUSH-04: yes) and now checked + Complete |

No orphaned requirements: exactly FLUSH-01..04 map to Phase 17 in REQUIREMENTS.md, all four appear in plan frontmatter, all four are checked.

### Decision Coverage

All trackable CONTEXT.md decisions are honored by shipped artifacts (16/16, gate non-blocking; `check.decision-coverage-verify` run this pass).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| .planning/ROADMAP.md | 234, 246 | `**Plans**: TBD` in the Phase 18 and Phase 19 sections | ℹ️ Info | Future-phase roadmap boilerplate (the native "not yet planned" state /gsd:plan-phase fills in), introduced by the v2.1 roadmap authoring commit ca74dd308 — predates Phase 17 entirely; the 17-08 closure diff touched only the Phase 17 progress lines (diff-printed above). No completion claim is unauditable; not a debt marker in the gate's sense |
| 17-08 evidence logs | — | none | — | No TBD/FIXME/XXX/TODO markers in any retained capture or modified record |

The three open code-review warnings and the #67/#68/#69 open windows noted in the prior report are unchanged backlog records (17-REVIEW-DISPOSITION ledger), outside this phase's must-haves — no new ones appeared.

### Human Verification Required

None. The one decision that required a human — the CR-07/#66 terminus plus the FLUSH-04 checkbox (the prior report's escalation items 1 and 2) — was taken and recorded by the maintainer at the 17-08 blocking-human checkpoint (Neil Skinner, 2026-11-13T14:42:08Z) and is carried in the overrides block above. No behavior-unverified truths remain; all runtime claims are backed by tests re-run live or retained captures inspected this pass.

### Gaps Summary

No gaps remain. The single prior blocking gap closed exactly as its own override invitation prescribed: the last named host-side capture avenue has a recorded outcome (section 8, NO-FIRE-IN-SERIES with honest weak-negative semantics), the maintainer ratified the explicit disposition with the 3-part D-15 rationale (section 9, verbatim who/when), and all five record surfaces plus REQUIREMENTS now carry that one decision consistently, with the prior report untouched outside the invited overrides block (hash-verified). The class itself is NOT claimed cured — the records state it stays live (~8%/run under the committed instrument), the reopener conditions survive, and the reproduction instrument stands wired; any future `fixed` vocabulary requires a cure pass under D-03's evidence bar. Everything else in the phase was already closed with live-verified pins at the unchanged pin 8f3ff11 (batteries re-run green this pass). Phase 17's requirement set is fully closed: FLUSH-01..04 all checked Complete.

---

_Verified: 2026-11-13T15:18:29Z_
_Verifier: Claude (gsd-verifier)_
