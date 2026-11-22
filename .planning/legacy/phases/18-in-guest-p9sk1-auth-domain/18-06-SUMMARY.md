---
phase: 18-in-guest-p9sk1-auth-domain
plan: "06"
subsystem: r9 kernel auth substrate (devcons hostowner gate + entropy probe) + harvey marker chassis
tags: [cr-01, cr-02, hostowner, iseve, eve-gate, eperm, ebadarg, rdrand, rdseed, cpuid, leaf1-ecx, scripted-cpuid, q35auth, hostowner-refused, push-before-pin]

requires:
  - phase: 18-02 (devcap #¤ + HOSTOWNER gate + q35auth substrate)
    provides: the #¤ device, per-proc user, eve gate the CR-01 fix protects, and the marker/venue machinery the refusal leg extends
provides:
  - "CR-01 closed: the #c/hostowner WRITE arm is eve-gated with the donor hostownerwrite semantics (iseve -> Eperm-class `devcons: hostowner write requires the hostowner` BEFORE the store lock; empty or >=128 bytes -> Ebadarg-class `devcons: bad arg to hostowner write`), proven by three devcons host pins AND live from ring 3 (the randread HOSTOWNER-REFUSED boot leg) — a capuse-mutated process can no longer re-gain eve by writing its own name to /dev/hostowner"
  - "CR-02 closed: armed_source() derives cpuid1_ecx from a fresh cpuid(1,0) call (leaf 7 stays the EBX source) over a scripted-cpuid cfg(test) seam with a loud unscripted-leaf miss — RDRAND-without-RDSEED hardware arms Rdrand instead of silently downgrading to jitter, and the two venue labels stay classification-correct through the probe (the declared KVM RDSEED precondition is now pinned, not load-bearing-by-luck)"
  - "r9 main pushed (b2a6151 -> f594b4c, origin/main == f594b4c) with the full battery green — the kernel bytes ready for 18-07's devip work and 18-08's single FOD pass (NO FOD re-wire, NO harvey flake.lock change this plan)"
affects: [18-07 (devip work rides the pushed pin), 18-08 (the phase's single FOD pass covers this plan's kernel bytes), Phase 19 cpu path (auth_chuid consumes the hardened eve boundary)]

actuals:
  tokens: 9200    # chars/4 over realized changes: r9 diff 20307c + harvey randread diff 2239c + evidence ~5100c + SUMMARY/docs ~9100c (estimate was 34000)
  tasks: 3
  commits: 1      # harvey production commits measured from the ledger 9a83a63c9..31b9b5c2d (randread leg); + r9 652f025, f594b4c (pushed); the docs commit follows
plan_head_before: 9a83a63c9063ec60c0a54f72613cc4fb630eadc1
plan_head_after: 31b9b5c2dcee992d731687fb65dd8dac7fe51c62   # last production commit; this SUMMARY's docs commit sits directly atop

tech-stack:
  added: []
  patterns:
    - "Scripted-CPUID seam (the test_tsc idiom extended): a cfg(test) (leaf,subleaf)->(eax,ebx,ecx,edx) store with a LOUD unscripted-leaf panic and a module-local cpuid wrapper — armed_source has ONE body for both builds and the probe->classify composition is host-testable per venue shape"
    - "Gate-before-store lock ordering: the iseve() check runs BEFORE taking the HOSTOWNER store lock because iseve itself reads HOSTOWNER under that lock (an mcslock self-deadlock otherwise) — documented at the gate"

key-files:
  created:
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-06-t2-red-evidence.json
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-06-t2-red-raw-libtest.json
    - .planning/phases/18-in-guest-p9sk1-auth-domain/deferred-items.md
  modified:
    - ../r9/port/src/devcons.rs
    - ../r9/xtask/src/main.rs
    - ../r9/x86_64/src/entropy.rs
    - build/x86_64/randread.c

key-decisions:
  - "The hostowner write gate fires BEFORE the HOSTOWNER store lock (iseve reads the static under its own lock — taking the store lock first would self-deadlock the mcslock); the refusal strings carry the donor's error classes (Eperm: `requires the hostowner`, Ebadarg: `bad arg`) in devcons's own voice"
  - "The donor's renameuser/kstrdup side effects are NOT ported (the plan's prohibition): no in-tree writer exists, so an eve writing a foreign name makes the base foreign — iseve() flips false for the writer (fail-closed), pinned and documented in hostowner_write_stores_for_eve; the reopener is the first real hostowner-write consumer"
  - "The TEST_SOURCE override retired entirely: with the scripted-cpuid seam, armed_source has one body for both builds and the fill-path pin scripts a jitter-venue CPUID — every entropy pin now exercises the real probe->classify composition (a source override tested nothing about which register fed which bit, exactly CR-02's blind spot)"
  - "The KVM evidence leg's rdrand label now carries its declared precondition as a TESTED fact (entropy_probe_venue_shapes_stay_correct scripts the KVM-host shape with RDSEED winning via the correctly-bound leaf-7 EBX) — the assumption-delta PROMOTE landed as planned"

patterns-established:
  - "Pattern: scripted-CPUID venue pins — any future CPUID-gated feature (the 13-03 lapic mode gate's family) can mirror test_cpuid to pin its venue shapes against scripted leaves instead of trusting the host CPU"
  - "Pattern: refusal-leg markers — a security gate's negative proof prints the refusal + errstr as the marker; a successful forbidden write is the fail-fast family (the exploit itself would be the FAIL line)"

requirements-completed: [AUTH-02, AUTH-03]   # declared by this plan; the shared-ID gate holds the REQUIREMENTS.md checkbox until sibling 18-08 (the co-declarer) also has a SUMMARY — correct, not force-marked

coverage:
  - id: D1
    description: "CR-01: the #c/hostowner write gate — iseve() Eperm-class refusal before the store lock + the donor 128-byte Ebadarg bound; eve path keeps replace-store; donor rename side effects unported (fail-closed divergence documented)"
    requirement: AUTH-03
    verification:
      - kind: unit
        ref: "cargo test -p port --lib -- devcons => 7 passed (the 3 new pins hostowner_write_refused_for_non_eve / hostowner_write_length_bound_refused / hostowner_write_stores_for_eve + the 4 pre-existing incl. user_eve_gate_default_and_hostowner_write unchanged); full port battery 146/146"
        status: pass
    human_judgment: false
  - id: D2
    description: "CR-01 non-regression: the boot-default eve base survives the gate — glenda stays eve by default, the four pre-existing randread cap legs and the random markers stay green, zero substrate: FAIL"
    requirement: AUTH-03
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --config q35auth (KVM default) PASS twice (Task 1 + the tracer feedback re-run): random: source=rdrand + READS-OK + POST-OK + USE-OK + ONESHOT-REJECT + HOSTOWNER-GATE + DONE, zero FAIL"
        status: pass
    human_judgment: false
  - id: D3
    description: "CR-01 live ring-3 proof: the randread HOSTOWNER-REFUSED leg — the mutated (non-eve) user writes its own name to #c/hostowner and the write refuses (a successful write fails fast naming the bypass)"
    requirement: AUTH-03
    verification:
      - kind: e2e
        ref: "boot log carries `cap: HOSTOWNER-REFUSED err=devcons: hostowner write requires the hostowner`; xtask CAP_HOSTOWNER_REFUSED_MARKER joins the finish() whole-capture list (SUBSTRATE-REFUSAL-GREEN, twice)"
        status: pass
    human_judgment: false
  - id: D4
    description: "CR-02: the RDRAND probe rebind — cpuid1_ecx from a fresh cpuid(1,0) call, leaf 7 supplies only EBX, classify + cpu.rs contracts byte-unchanged; the scripted-cpuid seam with a loud unscripted-leaf miss; TEST_SOURCE retired, the fill pin rides the composition"
    requirement: AUTH-02
    verification:
      - kind: unit
        ref: "RED: both probe pins failed on their planned assertions against the unfixed probe (RED_EVIDENCE_OK, evidence/18-06-t2-red-evidence.json + raw libtest JSON); GREEN: entropy filter 9/9 (6 pre-existing + 3 new); full x86_64 bins battery 86/86; REBIND-CLEAN (no cpuid(7,0) call binds a non-EBX register); git diff cpu.rs empty"
        status: pass
    human_judgment: false
  - id: D5
    description: "The venue-precondition harden (the assumption-delta PROMOTE): the KVM rdrand label's RDSEED-bearing-host precondition + the TCG jitter shape are pinned as scripted venue shapes through the probe"
    requirement: AUTH-02
    verification:
      - kind: unit
        ref: "entropy_probe_venue_shapes_stay_correct (KVM-host: RDSEED wins via leaf-7 EBX; TCG qemu64: neither bit -> Jitter); the boot gates' finish()-time venue assert untouched and green (source=rdrand on the KVM leg)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Battery + push posture: full port/x86_64 batteries green with the new pins, check+clippy clean, aarch64 byte-identical, r9 main pushed (push-before-pin; the pin advance + FOD pass belong to 18-08)"
    requirement: AUTH-03
    verification:
      - kind: other
        ref: "BATTERY-CLEAN (port --lib 146/146, x86_64 --bins 86/86, xtask check + clippy exit 0); aarch64 diff empty; PUSHED-CLEAN (origin/main == f594b4c, unpushed list empty); no flake.lock/flake.nix byte in either repo's commits this plan"
        status: pass
    human_judgment: false

duration: 21min
completed: 2026-11-14
status: complete
---

# Phase 18 Plan 06: CR-01/CR-02 Gap-Closure Summary

**The two critical review findings closed at the r9 source: the #c/hostowner write is eve-gated with the donor's bound (refused live from ring 3 — the capuse eve-base bypass is dead), and the entropy RDRAND probe reads leaf-1 ECX by construction over a scripted-cpuid seam — both pinned, battery green (146/86), aarch64 frozen, r9 pushed (f594b4c) with zero FOD or pin work.**

## Performance
- **Duration:** 21 min | **Started:** 2026-11-14T15:19:22Z | **Completed:** 2026-11-14T15:40:00Z | **Tasks:** 3/3 | **Files modified:** 7 (3 r9, 1 harvey code, 3 phase docs/evidence)

## Accomplishments
- CR-01 (T-18-14, critical EoP): the exploit chain is dead — a capuse-mutated process writing `norbert\n` to #c/hostowner now gets `devcons: hostowner write requires the hostowner` (proven live: `cap: HOSTOWNER-REFUSED err=...` on the q35auth boot, green twice), where before the ungated write re-gained eve and reopened #¤/caphash. The gate + the donor's 128-byte bound carry three deterministic host pins; the boot-default glenda-eve base and all four pre-existing cap legs are pinned non-regressed
- CR-02 (T-18-15, high info-disclosure): armed_source's probe is re-bound — cpuid(1,0)'s ECX feeds has_rdrand, leaf 7's EBX feeds has_rdseed — over a NEW scripted-cpuid cfg(test) seam (loud unscripted-leaf panic, one body for both builds). Both mis-wiring directions are RED-proven pins (the silent Jitter downgrade on RDRAND-only hardware; the #UD direction on a leaf-7-ECX-bit30 CPU); the KVM-host/TCG-qemu64 venue shapes stay classification-correct through the probe (the coincidental-reliance harden, declared + tested)
- The battery + push posture holds: port 146/146 (143 baseline + 3), x86_64 bins 86/86 (83 + 3), xtask check + clippy clean, aarch64 byte-identical at the v2.0 pin, r9 main pushed (652f025 + f594b4c, origin/main == f594b4c), NO flake.lock/FOD touch (18-08 owns the single pass)

## Task Commits
1. **Task 1: CR-01 gate + refusal pins + randread HOSTOWNER-REFUSED leg** — r9 `652f025` (fix; devcons gate + 3 pins + xtask marker) + harvey `31b9b5c2d` (fix; the capleg leg 5)
2. **Task 2: CR-02 rebind + scripted-cpuid seam + probe pins** — r9 `f594b4c` (fix; RED evidence RED_EVIDENCE_OK before the rebind, pins riding the one class commit per the plan's 17-04 precedent)
3. **Task 3: battery + aarch64 + push** — no new commits (the task's deliverable is the green battery + the push; both prior plans' commits are the payload), evidence in this SUMMARY

**Plan metadata:** this SUMMARY + STATE/ROADMAP/REQUIREMENTS (docs commit follows)

## Files Created/Modified
- r9 `port/src/devcons.rs` — the QID_HOSTOWNER write gate (iseve Eperm-class + 128-byte Ebadarg-class before the store lock), the module/test-comment updates, the hostowner_chan helper + 3 refusal pins
- r9 `x86_64/src/entropy.rs` — the probe rebind, the scripted-cpuid seam (test_cpuid store + cpuid wrapper), TEST_SOURCE retired, the fill pin rewired onto the composition, 3 probe pins
- r9 `xtask/src/main.rs` — CAP_HOSTOWNER_REFUSED_MARKER const, the finish() whole-capture list entry, the PASS println names the five cap markers
- harvey `build/x86_64/randread.c` — capleg leg 5 (the refusal write + `cap: HOSTOWNER-REFUSED err=` marker, fail-fast on success) + the five-marker file-top comment
- `evidence/18-06-t2-red-evidence.json` + `18-06-t2-red-raw-libtest.json` — the CR-02 RED evidence record (classified RED_EVIDENCE_OK) + the unmodified libtest JSON stream
- `deferred-items.md` — the devshell vendored-awk shadowing discovery (see Issues Encountered)

## Decisions Made
- The gate fires BEFORE the HOSTOWNER store lock (iseve reads the static under its own lock — a store-lock-first ordering would self-deadlock the mcslock); the refusal strings carry the donor's error classes in devcons's voice
- The donor's renameuser/kstrdup side effects stay unported per the plan's prohibition — fail-closed divergence (an eve writing a foreign name loses eve) pinned in hostowner_write_stores_for_eve; reopener: the first real hostowner-write consumer brings the donor's rename semantics
- TEST_SOURCE retired rather than kept: with the seam, a source override would leave the probe->classify composition untested exactly where CR-02 hid — the one fill-path pin that used it now scripts a jitter-venue CPUID

## TDD Gate Compliance
Task 2 (tdd="true") followed the 17-04 precedent the plan itself directs: RED evidence was captured and machine-classified (RED_EVIDENCE_OK, `gsd_run check tdd-red-evidence` over evidence/18-06-t2-red-evidence.json — both target pins failed on their planned assertions against the UNFIXED probe, 9 tests discovered, 7 passed, only the 2 planned targets failed) BEFORE the rebind landed; the pins then rode the ONE class commit `fix(18-06):` (r9 f594b4c) per the plan's own "one class commit" text — no separate test() commit exists for 18-06, same as 17-04. Task 1 is a tracer (not tdd): executed, verified, committed, then the tracer feedback gate re-ran BOTH verifies end-to-end green (devcons filter 7/7 + the full boot-test with the five markers) before expansion continued.

## Deviations from Plan

None - plan executed exactly as written.

### Environment note (not a deviation)
The substrate rebuild surfaced `mkimage.sh`'s auth-section resolver failing inside `nix develop` — the devshell's vendored Plan 9 awk (hostbin first on PATH) silently empties the TARG parse. Not caused by this plan's changes, not a gate break (the flake-app PATH shape is unaffected; verified rc=0 with GNU awk resolving first). Recorded in `deferred-items.md` with remediation options; mkimage.sh deliberately untouched (out of scope).

**Total deviations:** 0. **Impact:** none — the plan's file set and semantics landed exactly as specified.

## Issues Encountered
- The mkimage-in-devshell awk shadowing (above): worked around for this plan by prepending gawk's store dir to PATH for the bake; the substrate image (§13, what q35auth boots) was never affected — it bakes green in both environments
- `R9_HARVEY_DIR` must be exported for r9's xtask boot-test (documented in the tool's own error; no default since 05-CONTEXT D-10) — set explicitly on every invocation

## Authentication Gates
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- 18-07 (devip): rides the pushed pin f594b4c; the devcons/entropy work here is complete and cannot conflict (different files)
- 18-08 (the phase's single FOD pass): the kernel bytes from BOTH this plan and 18-07 land in its one two-derivation boot-iso re-wire + harvey flake.lock advance — deliberately untouched here (push-before-pin posture: r9 is pushed, the pin is not advanced)
- Phase 19 note: auth_chuid's eve boundary is now an actual security boundary (the write arm cannot re-gain eve); the secstore devip reopener is unaffected by this plan

## Self-Check: PASSED

All key files exist on disk (the three r9 sources, the harvey randread.c, both evidence JSONs, deferred-items.md); commits verified as ancestors of HEAD in their repos (r9: 652f025, f594b4c — the latter also == origin/main, unpushed list empty; harvey: 31b9b5c2d); aarch64 diff empty; flake.lock untouched in both repos; the boot log carries all five cap markers + the venue-correct armed-source label, zero substrate: FAIL.

---
*Phase: 18-in-guest-p9sk1-auth-domain*
*Completed: 2026-11-14*
