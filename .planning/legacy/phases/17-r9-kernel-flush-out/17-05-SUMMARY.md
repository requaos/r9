---
phase: 17-r9-kernel-flush-out
plan: 05
subsystem: kernel
tags: [r9, x86_64, namec, pgrp, union-mounts, sys-create, windows-18, windows-45, host-pins, fod-rewire]

requires:
  - phase: 17-r9-kernel-flush-out
    provides: the churn/composed contention captures at pins 76d34a873/75ababa53 (#18's under-load evidence), the 17-04 cap-walk tree at r9 63bfda9 (the audit baseline)
  - phase: 08-r9-tcp-ip-path-devip-compatible-net
    provides: WINDOWS #18 (the v1.0-era pgrp-lock row) + #45 (the union-create miss record)
provides:
  - WINDOWS #18 CLOSED as verify+pin — all four pgrp-lock sites in x86_64/src/syscall.rs already conform to the snapshot-then-drop contract (verbatim-quote audit in this SUMMARY); no code change needed
  - WINDOWS #45 CLOSED as fix+pin — sys_create's final-component probe and create dispatch consult the union chain (walk_one_union/create_chan_union, the per-consult chain_snapshot shape, first-accepting member, refusals propagate), RED-proven host pins (6) + devmnt failed-create fid clunk
  - the nomount follow-up (r9 421d5e1): the union consult never fires for #-rooted parents — the boot namespace's `bind -a /root /` owns the empty mount-table key a registry attach chan's empty path would collide with (the tier1 gate caught the over-reach as T1-FAIL date; root-caused live to init's silently-failing setenv("#e/timezone"))
  - the namec host-pin home: port/src/namec_tests.rs (crate-root, the devroot_rawimg/devenv precedent extended — venue A)
  - the honest record that the tier1 date check's green rests on a pre-existing ctime(0)+EST display quirk (deferred; identical pre/post)
affects: [17-06, 17-07, VERIFICATION (walks 17-FLUSHOUT.md — the #18/#45 rows filled), Phases 18/19 (the namespace path they hammer is proven under contention)]

actuals:
  tokens: 13766   # chars/4 over the realized diff: Harvey 22633/4 + r9 31270/4 + deferred-items 1162/4
  tasks: 3
  commits: 5      # measured: Harvey a1d9e7a61..HEAD (2 pin advances + ledger + SUMMARY docs); r9 4 more (its own repo)

tech-stack:
  added: []
  patterns: ["the union consult as a twin, not a rewrite: frozen-arch callers keep the head-only walk_one/create_chan signatures; the union-aware twins carry namec's #-arm nomount bit explicitly — an empty-path registry attach chan and the boot root mount share the empty mount-table key, and only the caller knows which is which", "the over-reach counterfactual as a test: the nomount pin asserts the WRONG behavior occurs with nomount=false, so the bit can never silently rot", "gate-driven re-derivation: an extra canary gate (tier1, beyond the plan's named two) caught the over-reach the named gates missed — smoke and q35shell boot through the chain image whose setenv flow differs from the tier image's init cpenv"]

key-files:
  created: [../r9/port/src/namec_tests.rs, .planning/phases/17-r9-kernel-flush-out/17-05-deferred-items.md, .planning/phases/17-r9-kernel-flush-out/evidence/17-05-red-evidence.json]
  modified: [../r9/port/src/namec.rs, ../r9/port/src/devmnt.rs, ../r9/port/src/lib.rs, ../r9/x86_64/src/syscall.rs, ../r9/flake.nix, flake.lock, .planning/WINDOWS.md, .planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md]

key-decisions:
  - "#18 closed verify+pin, no fix: the audit found every site already in shape — the shared namec chain_snapshot (:206-212) carries the contract and the four syscall.rs sites lock only the mount-table mutation"
  - "#45's fix is a twin, not a signature change: aarch64 (frozen) calls walk_one/create_chan with today's shapes, so the union consult lives in walk_one_union/create_chan_union and x86_64's sys_create routes through them"
  - "create refusal semantics: only the structural 'device has no create' arm advances the union chain — a member whose create op runs and FAILS propagates (the 15-03 server-owns-the-policy stance; a refusing server must error, never silently land the file in a shadowed member — the T-17-10 wrong-member class).  Deliberate deviation from donor createdir's MCREATE gate, documented at the fn"
  - "the nomount bit is caller-owned: sys_create re-derives path[0]=='#' exactly as namec's own '#' arm does; the twins cannot tell an empty-path registry attach from the boot root (both key to \"\")"
  - "the devmnt failed-create fid clunk promoted from latent-leak to load-bearing: create_chan_union tries members in chain order, so a refusing member's clone fid must clunk, not leak"

patterns-established:
  - "Empty-path keying: every union consult must know whether the parent is #-rooted; pgrp::normalize('/')==\"\" is the same key a registry attach chan carries by default — document at every new consult site"
  - "The collision counterfactual pin: when a guard bit exists because of a measured collision, pin the collision itself (nomount=false errors) so removing the bit reds a test"

requirements-completed: [FLUSH-04]

coverage:
  - id: D1
    description: "#18's four-site pgrp-lock audit (verify+pin): every site drops the lock before any device walk / 9P RPC"
    requirement: FLUSH-04
    verification:
      - kind: manual_procedural
        ref: "The verbatim-quote audit table in this SUMMARY (four rows: acquisition / drop point / first post-drop device op) + the site-count grep == 4 at :527/:599/:716/:1159 + the 17-02 contention captures (39 RFMEM sem generations + /bin/cat x1400 churn, green twice)"
        status: pass
    human_judgment: false
  - id: D2
    description: "#45's union consult: sys_create's probe and create dispatch reach the first-accepting union member, RED-proven host pins"
    requirement: FLUSH-04
    verification:
      - kind: unit
        ref: "cargo test -p port --lib namec = 6/6 (walk_one_union_finds_nonhead_member, create_chan_union_reaches_accepting_member, create_chan_union_refusal_propagates_not_falls_through, create_chan_union_nomount_never_consults_the_root_key, the two no-mounts regression pins); RED_EVIDENCE_OK evidence/17-05-red-evidence.json; full battery 120/120; x86_64 battery 76/76"
        status: pass
      - kind: integration
        ref: "boot gates at the final pin 09b59f6f2 (r9 59d28fa): cargo xtask boot-test --config q35shell PASS x2, nix run .#boot-r9-smoke-x64 exit 0 x2, nix run .#boot-r9-tier1-x64 ALL STAGES PASS (38 T1-OK incl. date — the gate that caught the over-reach), nix run .#boot-r9-net-smoke-x64 ALL STAGES PASS"
        status: pass
    human_judgment: false
  - id: D3
    description: "The push-before-pin lineage: one class commit + the nomount follow-up, two FOD re-wires (two-derivation proofs), own-commit pin advances"
    verification:
      - kind: integration
        ref: "R9-CLEAN-PUSHED + PIN-COMMIT-CLEAN verified after each cycle (r9 origin/main..HEAD = 0; Harvey pin commits 0466642c6/09b59f6f2 flake.lock-only); FOD cycle 1: ISO file sha256 08f62ea8… byte-identical two derivations, NAR sha256-RZEFjz… three-way; cycle 2: file sha256 4f99894f…, NAR sha256-jlHUrP…; nix build .#boot-iso + --rebuild green at each wired value"
        status: pass
    human_judgment: false
  - id: D4
    description: "Ledger closure: WINDOWS #18/#45 with native resolve fields (JSON + table), FLUSHOUT rows filled, no collateral rows"
    verification:
      - kind: manual_procedural
        ref: "Task-3 gates: ROWS-18-45-CLOSED, REASON-CITES-SHA, FLUSHOUT-NAMEC-FILLED all echoed; git diff on WINDOWS.md touches only the two rows + the maintained counts"
        status: pass
    human_judgment: false

# Metrics
duration: 97 min
completed: 2026-11-13
status: complete
---

# Phase 17 Plan 05: namec/pgrp — #18 Verify + #45 Union-Create Summary

**#18 closed on a four-site verbatim-quote audit (every pgrp-lock site already snapshot-then-drop — zero code change); #45 closed with the union consult for sys_create (walk_one_union/create_chan_union, first-accepting member, RED-proven 6 host pins) — and the fix's own over-reach was caught by the tier1 gate and root-caused live to the boot namespace's empty mount-table key, taking a nomount follow-up commit and two full FOD cycles to green.**

## Performance

- **Duration:** 97 min
- **Started:** 2026-11-13T00:14:15Z
- **Completed:** 2026-11-13T01:51:08Z
- **Tasks:** 3/3
- **Files modified:** 12 (Harvey 6 + r9 6)

## Task 1 — the #18 Four-Site Audit (verify+pin; no code change)

`grep -nE '(pg|pgrp)\.lock' x86_64/src/syscall.rs` = **4 sites at HEAD**, exactly as baselined (:527/:599/:716 as `pg.lock` + :1159 as `proc.pgrp.lock`). Every site conforms to the snapshot-then-drop contract — the LOCKING comment's shape (namec.rs:130-138) carried at each:

| Site | Function | Lock acquisition (verbatim) | Locked region | Drop point | First post-drop device/RPC op | Verdict |
|------|----------|------------------------------|---------------|------------|-------------------------------|---------|
| :527 | sys_bind | `let mut guard = pg.lock(&node);` | `guard.bind(src, &tgt, flag)` — pgrp.rs table insert only (chan dup + Vec insert; no device walk, no 9P RPC) | `drop(guard);` (:529) | `port::namec::close_chan(&tgt);` (:531 — can Tclunk a DEV_MNT) runs AFTER the drop | conforms |
| :599 | sys_unmount | `let mut guard = pg.lock(&node);` | `guard.unmount(&tgt.path, from.as_ref())` — table removal only | `drop(guard);` (:601) | `port::namec::close_chan(&tgt);` (:602) | conforms |
| :716 | sys_mount | `let mut guard = pg.lock(&node);` | `guard.bind(mnt_chan, &tgt, flag)` — table insert only (the blocking fversion/attach/namec all ran pre-lock, per the :699 comment) | `drop(guard);` (:718) | `port::namec::close_chan(&tgt);` (:719) | conforms |
| :1159 | sys_create (no-slash arm) | `let guard = proc.pgrp.lock(&node);` | `guard.chain_snapshot(&d.path)` — the snapshot itself; the guard's scope IS the block expression | block-end (:1161 — the guard drops at the block's close) | `port::namec::redirect_clone(&head.to)` (:1166 — a DEV_MNT clone_chan Twalk RPC) runs OUTSIDE the block | conforms (the snapshot-then-drop shape verbatim) |

**Closure mode: verify + pin.** The contention evidence: the 17-02 captures — the q35churn profile's 39 RFMEM semacquire/semrelease generations (4 contenders × 200 cycles, spawned nowait so they overlap following execs) and the composed /bin/cat ×1400 fork+exec churn (real mounted-binary exec under a memory bound), both green twice — are exactly the rc-pipeline/backgrounded-command contender shape the row named, with no wedge. Task 1 produced no code delta (its verify gates: the count grep == 4; the battery only-if-fixed — not triggered).

## Task 2 — #45: the Union Consult (TDD, one class + one gate-forced follow-up)

**RED** (stub state — the twins pass through to the head-only walk_one/create_chan): 3 target tests failed on planned assertions, `walk_one_union_finds_nonhead_member` at "the probe must consult the union chain (the #45 fix): devroot: name not found" and `create_chan_union_reaches_accepting_member` at "the create must reach the accepting union member (the #45 fix): namec: device has no create" — **the original #45 error string verbatim**. Machine-classified `RED_EVIDENCE_OK` (evidence/17-05-red-evidence.json, TAP conversion of the real libtest JSON; raw preserved beside it).

**GREEN** (r9 `cfe7af0`, the class commit):
- `walk_one_union(c, name, pg, nomount)` — snapshots the chain under the per-consult pgrp lock, drops it, walks each member in chain order (first-accepting wins), mirroring the multi-component walker at :206-212; `nomount` short-circuits to the head-only `dev_walk`.
- `create_chan_union(c, name, mode, perm, pg, nomount)` — follows the chain to the first member whose device accepts a create (DEV_ENV/DEV_SRV/DEV_MNT); **only the structural no-create arm advances; a member whose create runs and fails PROPAGATES** (the 15-03 server-owns-the-policy stance — refusing servers error, never silently land files in shadowed members; deliberate deviation from donor createdir's MCREATE gate, documented at the fn).
- devmnt::create's failed-create clone fid now clunks best-effort — load-bearing once members are tried in chain order.
- x86_64 sys_create re-wired to the twins; **aarch64 untouched** (frozen — the head-only `walk_one`/`create_chan` keep their signatures; that freeze is WHY the fix is a twin, not a signature change).
- The pin home: port/src/namec_tests.rs (crate-root mod, venue A — the devroot_rawimg/devenv precedent extended: synthetic paqfs image + the stub env-group hook). 6 pins green; batteries port 120/120 (baseline 114), x86_64 76/76; clippy clean for the new files.

**The over-reach the plan's fails_when anticipated** ("the union consult changed namespace resolution for the boot chain — the fix over-reached; re-derive against the failure") — caught not by the plan's two named gates (q35shell and smoke both PASSED at the first advance) but by **tier1, run as an extra namespace canary**: `T1-FAIL date`, reproducible, and A/B-clean at the old pin. Live-probe root cause (typed diagnostics against hand-composed ISOs both sides): a registry attach chan (devenv's root) carries an **empty path**, and the boot namespace's `bind -a /root /` owns the empty mount-table key (`pgrp::normalize("/") == ""`) — so `init`'s `cpenv` → `setenv("#e/timezone")` → sys_create consulted the ROOT union and died on its members' no-create, silently dropping the timezone env. The old kernel's green was itself an accident (see the honest record below). **Fix (r9 `421d5e1`)**: namec's `#`-arm nomount bit flows through — sys_create derives `path[0] == b'#'` exactly as namec's own '#' arm does; the twins skip the consult. The pin `create_chan_union_nomount_never_consults_the_root_key` asserts BOTH the fix and the counterfactual (nomount=false errors on the collision — the bit can never silently rot). Verified: tier1 ALL STAGES PASS against the fix tree before commit; the full battery at the final pin below.

## The Honest Record: the tier1 date check was never really testing date

`hdate -n` prints **0** on every r9-x64 kernel this era (`/dev/bintime` — which 9k's devcons serves — is absent from r9's, so libc `nsec()` returns 0), and the sweep's `*20[0-9][0-9]*` expectation is met only because `ctime(0)` under the EST timezone data displays "Sun Feb  7 01:28:16 EST 2006". Measured live on BOTH sides: identical at r9 63bfda9 (pre-17-05) and r9 59d28fa (final) — `hdate -n` = 0 at both. 17-05's regression merely removed the timezone env (display fell to "Jan 1 1970 GMT", redding the gate); the nomount fix restored the status quo ante, byte-for-byte display included. Recorded open in 17-05-deferred-items.md — the root fix belongs to whichever plan first owns the clock seam (Phase-14's recorded no-RTC era shape).

## The Pin-Advance Lineage (two full cycles, push-before-pin each)

- r9 `cfe7af0` fix(17-05): the union consult class commit (tests + fix + sys_create re-wire + the devmnt clunk)
- r9 `e4506f5` fix(17-05): **FOD re-wire #1** — kernel A plain + kernel B (`namec45-proof-b`) byte-identical (file sha256 `08f62ea8401611ed09f1e1f1df805f45f4d292edf16f83bd875747676bf66660`, NAR `sha256-RZEFjz7m+LgTA0z06wAtEEGJascR0/52v++iYrhbxDI=` = `4591058f…`, three-way with the FOD's own got-value); build + --rebuild green
- Harvey `0466642c6` chore(17-05): pin → e4506f5 (lock-only) — **the tier1 red landed here**, caught in the re-green pass
- r9 `421d5e1` fix(17-05): the nomount follow-up (the #-arm bit + the counterfactual pin)
- r9 `59d28fa` fix(17-05): **FOD re-wire #2** — kernel A plain + kernel B (`namec45b-proof-b`) byte-identical (file sha256 `4f99894f90bf3c4172d4443b1c1832336f195deccbae3222f74015dd0f4abe7d`, NAR `sha256-jlHUrPX26vmTX8uW9HkrcklNcKVKUjCtdS3EfYIbMEs=` = `8e51d4ac…`, three-way); build + --rebuild green
- r9 pushed `63bfda9..59d28fa` BEFORE each Harvey advance (origin/main..HEAD = 0 verified at each)
- Harvey `09b59f6f2` chore(17-05): pin → 59d28fa (lock-only) — the final pin

**Gates at the final pin:** q35shell PASS ×2 · boot-r9-smoke-x64 exit 0 ×2 (ALL STAGES PASS) · boot-r9-tier1-x64 ALL STAGES PASS (38 T1-OK incl. date) · boot-r9-net-smoke-x64 ALL STAGES PASS.

## Task Commits

1. **Task 1** — audit-only, no code delta (the audit table above is the deliverable; verification = the count grep + the battery baseline)
2. **Task 2** - Harvey `0466642c6`, `09b59f6f2` (chore: the two lock-only pin advances); r9 `cfe7af0` + `421d5e1` (the class commit + the nomount follow-up) + `e4506f5`/`59d28fa` (the two FOD re-wires)
3. **Task 3** - `1629f9301` (docs: WINDOWS #18/#45 closed in JSON + table with the native resolve fields; FLUSHOUT rows filled; the RED evidence committed)

## TDD Gate Compliance

Task 2 carried tdd="true": **RED** captured and classified BEFORE any implementation (`17-05-red-evidence.json`, `RED_EVIDENCE_OK`, target test executed and failing on its planned assertion — the #45 error string verbatim); **GREEN** via the class commit. Per the plan's own instruction ("Land everything … as ONE namec/pgrp class commit") the pins rode the class commit — no separate `test(17-05)` commit exists in r9 history, the same documented shape as 17-04. The nomount follow-up's new pin (`create_chan_union_nomount_never_consults_the_root_key`) was written RED-first against the broken intermediate pin (its acceptance fails there) and GREEN in `421d5e1`; its RED was not separately classified (the failure it pins was already measured live by the tier1 gate — the stronger evidence).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The union consult over-reached into #-rooted parents**
- **Found during:** Task 2 (the gate re-green pass — tier1 run as a namespace canary beyond the plan's two named gates)
- **Issue:** A registry attach chan's empty path collides with the boot namespace's `bind -a /root /` empty mount-table key; init's setenv("#e/timezone") consulted the ROOT union and failed silently (the tier1 gate's T1-FAIL date; the plan's own fails_when named this exact class)
- **Fix:** the nomount bit flows from sys_create into both twins (r9 `421d5e1`); the counterfactual pin asserts the collision stays real
- **Files modified:** ../r9/port/src/namec.rs, ../r9/x86_64/src/syscall.rs, ../r9/port/src/namec_tests.rs
- **Verification:** tier1 ALL STAGES PASS at the final pin; A/B against the old pin isolated the cause; the live typed-probe transcript (hdate/hdate -n/ls /env both kernels) named the mechanism
- **Committed in:** r9 421d5e1

**2. [Rule 1 - Bug] devmnt::create leaked the clone fid on a refused create**
- **Found during:** Task 2 (design review of the multi-member try — pre-existing for the single head, load-bearing the moment members are tried in order)
- **Fix:** best-effort clunk of newfid on the create error tail (r9 `cfe7af0`)
- **Files modified:** ../r9/port/src/devmnt.rs
- **Verification:** code-path review (the clunk mirrors clunk_if_last's best-effort shape); the boot gates exercise refusing members through the tier image's read-only paqfs head

**3. [Rule 3 - Blocking] The plan's "ONE class commit" became class + follow-up**
- **Found during:** Task 2 (the tier1 red after the first pin advance)
- **Issue:** the first advance was already pushed; amending would need a force-push (prohibited)
- **Fix:** the follow-up commit `421d5e1` + a second full FOD cycle + a second pin advance — the plan's own re-derivation path ("re-derive against the failure"), executed with the standard discipline

**4. Plan-shape note (not a defect): the union arm is a twin, not an edit of create_chan**
- The plan's acceptance criterion names "create_chan's union-head arm (:390)"; the frozen aarch64 caller forces signature preservation, so the consult lives in `create_chan_union` and x86_64's sys_create routes through it. Same contract, frozen-compatible shape — documented at both fns.

---

**Total deviations:** 2 auto-fixed bugs + 1 blocking-shape note + 1 plan-shape note
**Impact on plan:** The over-reach fix was the plan working as designed (its fails_when predicted the class; the gate caught it). The extra canary gate (tier1) is now part of 17-05's evidence standard for namespace changes. No scope creep — the date-quirk discovery is recorded, not chased.

## Issues Encountered

- The plan's two named gates (q35shell + smoke) both PASSED on the broken intermediate pin — the tier image's init-cpenv flow is the only boot path that exercises the colliding create; the composed-gate family's diversity (not just the named two) is what makes the battery a battery
- The FOD needed two cycles (the follow-up re-staled the first re-wire within the plan) — both with the full two-derivation proof
- Debugging required hand-composed ISOs with typed expect probes on both kernels (the sweep's backquotes swallow date's output; `hdate -n` was the decisive probe)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 17-06/17-07 proceed on the union-aware namec at pin 09b59f6f2 (r9 59d28fa); FLUSHOUT carries the filled #18/#45 rows
- The empty-path mount-key collision is now a documented pattern (namec.rs + the counterfactual pin) — any future consult site must carry its own nomount derivation
- The clock-seam finding (tier1 date's accidental green) is open in 17-05-deferred-items.md for whichever plan owns the epoch clock

## Self-Check: PASSED

- Created/modified files exist on disk (SUMMARY, deferred-items, both RED-evidence files; r9 namec_tests.rs, namec.rs, devmnt.rs, lib.rs, syscall.rs, flake.nix)
- Commits verified ancestors of HEAD: Harvey 0466642c6, 09b59f6f2, 1629f9301 (+ this SUMMARY's docs commit); r9 cfe7af0..59d28fa all on origin/main (push-before-pin verified at each cycle)
- Plan-level verification re-run post-commit: site-count grep == 4; port --lib 120/120; q35shell PASS x2; smoke exit 0 x2; tier1 ALL STAGES PASS (38 T1-OK incl. date); net-smoke ALL STAGES PASS; R9-CLEAN-PUSHED + PIN-COMMIT-CLEAN; Task-3 gates ROWS-18-45-CLOSED + REASON-CITES-SHA + FLUSHOUT-NAMEC-FILLED
- commits: measured 3 pre-SUMMARY (a1d9e7a61..HEAD: the two pin advances + the ledger closure), the SUMMARY docs commit lands as the 4th

---
*Phase: 17-r9-kernel-flush-out*
*Completed: 2026-11-13*
