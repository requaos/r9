---
phase: 18-in-guest-p9sk1-auth-domain
plan: "08"
subsystem: infra (FOD/pin integrity chain) + auth (secstore round-trip gate) + records closure
tags: [r9, fod, pin-advance, secstore, round-trip, gate-flip, presence-demands, records-closure, auth-08, lock-only]

requires:
  - phase: 18-06 (CR-01/CR-02 kernel fixes, r9 652f025 + f594b4c)
    provides: the kernel bytes the FOD pass carries
  - phase: 18-07 (devip two-write fix, r9 ab4cb66)
    provides: the TCP delivery the restored secstore legs ride
provides: [advanced pin green twice, secstore round-trip proven on delivered bytes, auth gate presence demands, closed phase records]
affects: [phase-19 cpu path, CI]

actuals:
  tokens: 17800   # chars/4 over realized changes: harvey diff 56956c (evidence logs included) + r9 flake.nix diff 1619c + this SUMMARY ~12700c (estimate was 46000)
  tasks: 3
  commits: 3      # measured: git rev-list --count dee637069..HEAD (pin eaf4879fe lock-only + gate flip 946da6635 + records 2e641f03b) + r9 9103522 (the FOD re-wire, its own repo, pushed)
plan_head_before: dee6370693cdaa13f8ca2cfaa68d849540333ba6
plan_head_after: 2e641f03bff9c3f81b61fbc1a34c33914b1f0aca   # last production/docs commit; this SUMMARY's metadata commit sits directly atop

tech-stack:
  added: []
  patterns:
    - "Presence-demand gate flip (the inverse of the named-SKIP arm): the retired skip's absence-assert becomes six presence stages + a string-first post-pass with named reds — string first, never string match, because the bracketed syslog tokens ([norbert] PUT) would be character classes under glob matching"
    - "Stubbed no-qemu expect harness (procs shadowing spawn/expect/exec/exit) validates a flipped gate's syntax AND its post-pass logic against positive/negative fake captures before the first real boot cycle burns minutes"

key-files:
  created:
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-08-auth-a.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-08-auth-b.log
  modified:
    - ../r9/flake.nix        # the FOD re-wire (its own commit 9103522, pushed)
    - flake.lock             # r9 b2a6151 -> 9103522 (lock-only commit eaf4879fe)
    - build/x86_64/auth-sweep.rc        # leg 9d restored, SKIP arm retired, rtstat in the ladder
    - build/x86_64/smoke-r9-auth.expect # six presence stages + the flipped post-pass
    - .github/workflows/r9-shell.yml    # the auth job comment corrected (IN-01)
    - build/x86_64/README.md            # blocker closed as DELIVERED + the round-trip marker rows + the -s venue fact
    - .planning/REQUIREMENTS.md         # AUTH-02/03/08 checked with cites + traceability Complete
    - .planning/STATE.md                # the 18-05/AUTH-08 blocker RESOLVED + Current Position closed

key-decisions:
  - "The FOD pass covers 18-06+18-07 bytes TOGETHER (one re-wire over the whole gap set — a re-wire over a partial tree stales again): two independent derivations through the non-FOD lib.bootIso (kernel A plain at ab4cb66 + kernel B at 27a141ac, a kernel-neutral marker commit in a discarded clone that forces a genuinely fresh kernel build), byte-identical ISOs, NAR sha256 f2e2fb99191a6f2c4f0fb36217e77bd8de4af05d33cd6d7ad4f952716bedbeb3 (sha256-8uL7mR…) three-way-agreed with the FOD's own got-value"
  - "The pin advance is lock-only and never bundled (INT-06): r9's FOD re-wire is its own pushed commit; Harvey's flake.lock advance (eaf4879fe) touches flake.lock and nothing else — verified commit-to-commit"
  - "The verdict fires ONLY on delivered bytes (D-14 held): AUTH-SECOBS + AUTH-OK secstore-roundtrip print only from the grep of AUTH-ROUNDTRIP-PAYLOAD-18-05 out of the -G GET's captured stdout — and the gate now demands that verdict PRESENT (the 18-05 absent-assert would red this gate by design)"
  - "The -s 127.0.0.1 venue pin (measured, first gate run): the secstore client's default serve $attr -> tcp!$auth!5356 rides cs's $attr translation, whose system-identity substrate ENOETs on r9 (the kernel's `namec: does not exist' relayed verbatim as the cs Rerror); the literal-IP dial class is proven on this venue every run (hauthprobe), so the client pins secstored's loopback listener with its own -s flag"
  - "The flip ADDS presence demands, never substitutes: the six new stages + the flipped post-pass sit beside every pre-existing tooth — nvram-prompt-absent, zero-AUTH-FAIL, the DEMO label, ANNOUNCE, SPEAKSFOR-DENIED-exactly-once, tr-ok, no-speaks-for — all still armed, and the T-18-13 redaction still runs LAST over the true capture"

requirements-completed: [AUTH-01, AUTH-02, AUTH-03, AUTH-04, AUTH-05, AUTH-06, AUTH-07, AUTH-08, AUTH-09]   # plan frontmatter verbatim; the shared-ID gate releases AUTH-02/03/08 with this SUMMARY — the gate verb's verdict governs the actual checkbox flips

coverage:
  - id: D1
    description: "The phase's ONE FOD pass over all 18-06+18-07 kernel bytes: the two-derivation byte-identity proof, the re-wire, the push"
    requirement: AUTH-02
    verification:
      - kind: other
        ref: "derivation A (plain, ab4cb66) ISO /nix/store/khlbbk7x… + derivation B (marker clone 27a141ac, fresh kernel-x64-27a141a build) ISO /nix/store/rvcbfplj…: cmp clean, file sha256 a1cffdb4564d1906b704d14b5e4378ba79eab2b54f44957e377b3b90e4e3ac0a both; NAR sha256-8uL7mRkabyxPD7NiF+d72N5K8F0zzW161PlScWvtvrM= both == the FOD's own got-value (the stale-hash mismatch report); FOD-REWIRE-GREEN (build + --rebuild at the wired hash, re-verified from the committed pushed rev); r9 9103522 pushed, PUSHED-CLEAN"
        status: pass
    human_judgment: false
  - id: D2
    description: "The Harvey pin advance lock-only + the strongest pin consumer re-green"
    requirement: AUTH-03
    verification:
      - kind: integration
        ref: "PIN-LOCK-ONLY (the pin commit's commit-to-commit diff is flake.lock alone); LOCK-EQUALS-R9-ORIGIN-MAIN (flake.lock r9 rev == 9103522c273a0ee1a05c6218c6224a56bc881720 == r9 origin/main); nix run .#boot-r9-net-smoke-x64 exit 0 — NET-SMOKE-GREEN, ALL STAGES PASS at the pin (/tmp/net-smoke-1808.console)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The restored 9d legs with the SKIP arm retired: venue copy, PUT with CWD at the venue, -G GET, the bytes-back grep -> AUTH-SECOBS + the verdict"
    requirement: AUTH-08
    verification:
      - kind: e2e
        ref: "capture: `secstore: saved 401 bytes' (the client PUT's own line) + `AUTH-SECOBS AUTH-ROUNDTRIP-PAYLOAD-18-05' (line 528 of both green captures) + `AUTH-OK secstore-roundtrip'; ROUNDTRIP-GREEN (all six markers + AUTH-DONE, zero AUTH-FAIL) + SKIP-ARM-RETIRED (zero AUTH-SKIP in the capture; the sweep grep shows no emission path — comment mentions only)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The flipped gate: six presence stages + the flipped post-pass with named reds, every pre-existing tooth still armed, the redaction last; the CI comment corrected (IN-01)"
    requirement: AUTH-08
    verification:
      - kind: integration
        ref: "stubbed no-qemu harness over the committed expect file: positive control (all-marker fake capture) -> full source parse + every post-pass green + STUBBED-EXIT-0; negative control (fake minus [norbert] GET) -> the named red `round-trip marker absent — secstored's [norbert] GET syslog token' + STUBBED-EXIT-1; live: gate exit 0 at the advanced pin (all stages incl. the six new + every post-pass)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Green twice consecutively at the advanced pin with both captures retained, redacted, and marker-complete — every AUTH truth re-asserted at the new pin"
    requirement: AUTH-01
    verification:
      - kind: e2e
        ref: "FULL-GATE-GREEN-TWICE + SKIP-FREE-BOTH + MARKERS-VERIFIED (19-marker loop over BOTH captures: the five AUTH-OK legs + round-trip set + PAIRED DEMO + ANNOUNCE + tr-ok + no-speaks-for + SPEAKSFOR-DENIED); evidence/18-08-auth-{a,b}.log retained, 550 lines each, `<demo-pw-redacted>' present in both, the current bake's password grep-absent from both"
        status: pass
    human_judgment: false
  - id: D6
    description: "The records closed: REQUIREMENTS (AUTH-02/03/08 + cites, history preserved), STATE (blocker resolved + closure decisions), README (blocker paragraph closed, no promise of wiring that does not exist)"
    requirement: AUTH-09
    verification:
      - kind: integration
        ref: "REQ-CLOSED (three checkboxes checked, each with its closure cite; AUTH-01/04/05/06/07/09 rows untouched — OTHER-ROWS-UNTOUCHED, the gate verb owns them); STATE carries no open 18-05/AUTH-08 blocker (RESOLVED entry with the fix + pin + evidence cites); README AUTH section documents the closed round-trip + the -s venue fact"
        status: pass
    human_judgment: false

duration: 33min
completed: 2026-11-14
status: complete
---

# Phase 18 Plan 08: The FOD Pass + Secstore Round-Trip + Phase Records Closure Summary

**The phase's three verification gaps closed end-to-end: the ONE FOD pass pinned all 18-06+18-07 kernel bytes (two-derivation proof three-way-agreed, r9 pushed at 9103522, Harvey flake.lock advanced lock-only, net-smoke re-green), the secstore round-trip is RESTORED and green twice at the pin on delivered bytes (AUTH-SECOBS + the verdict presence-demanded, the SKIP arm retired), and every record names the closure (REQUIREMENTS AUTH-02/03/08, the STATE blocker, the README, the CI comment).**

## Performance
- **Duration:** 33 min | **Started:** 2026-11-14T16:34:18Z | **Completed:** 2026-11-14T17:07Z | **Tasks:** 3/3 | **Files modified:** 10 (1 r9, 9 harvey incl. evidence)

## The Two-Derivation Proof (the record)

| Derivation | Tree state | Kernel build | ISO realization |
|---|---|---|---|
| A (plain) | ab4cb66 (r9 main) | cached at the same drv | non-FOD `lib.bootIso` → /nix/store/khlbbk7x…-r9-boot-iso-ab4cb66 |
| B (independence) | 27a141ac (marker clone: ab4cb66 + a kernel-neutral README marker commit, discarded) | FRESH (r9-kernel-x64-27a141a.drv built) | non-FOD `lib.bootIso` → /nix/store/rvcbfplj…-r9-boot-iso-27a141a |
| The FOD itself | ab4cb66, stale wired hash | same | the mismatch report's got-value |

- **Byte identity:** `cmp` clean; file sha256 `a1cffdb4564d1906b704d14b5e4378ba79eab2b54f44957e377b3b90e4e3ac0a` on both ISOs
- **Three-way NAR agreement:** A == B == the FOD's own got-value == `sha256-8uL7mRkabyxPD7NiF+d72N5K8F0zzW161PlScWvtvrM=` (base16 `f2e2fb99191a6f2c4f0fb36217e77bd8de4af05d33cd6d7ad4f952716bedbeb3`)
- Wired at r9 `9103522` (its own `fix(18-08)` commit with the lineage comment); `nix build .#boot-iso` + `--rebuild` green, re-verified from the committed pushed rev; PUSHED-CLEAN
- Harvey pin advanced `b2a6151 -> 9103522` in `eaf4879fe` — lock-only, verified commit-to-commit

## Accomplishments
- The integrity chain is whole again: every composed gate now consumes the CR-01/CR-02/devip-fix kernel bytes through the pin, proven by the net-smoke gate's ALL STAGES PASS at the advanced pin
- AUTH-08's client half delivered: the PUT/GET round-trip runs as a demanded leg whose verdict is emittable ONLY from the payload line coming back in the -G GET's delivered bytes — green twice consecutively at the pin, both captures retained (redacted, password provably absent)
- The gate flip has teeth and only teeth ADDED: six presence stages + a named-red post-pass, with every pre-existing tooth (nvram-prompt-absent, zero-AUTH-FAIL, DEMO label, ANNOUNCE, DENIED-exactly-once, tr-ok, no-speaks-for) still armed and the T-18-13 redaction still last — proven by a stubbed positive/negative control pair before the first live boot
- The records close coherently: REQUIREMENTS AUTH-02/03/08 checked with cites (history preserved), the STATE 18-05/AUTH-08 blocker RESOLVED, the README blocker section rewritten as DELIVERED (with the new -s venue fact), and the CI comment states the true contract (IN-01 discharged, PENDING FIRST PUSH posture kept)

## Task Commits
1. **Task 1: the FOD pass** — r9 `9103522` (fix; the hash re-wire + lineage comment, pushed) + harvey `eaf4879fe` (chore; the lock-only pin advance)
2. **Task 2: the restored legs + the flipped gate** — harvey `946da6635` (feat; sweep 9d + expect flip + CI comment)
3. **Task 3: green twice + records** — harvey `2e641f03b` (docs; evidence + REQUIREMENTS + STATE + README)

**Plan metadata:** `pending` (docs: this SUMMARY + STATE/ROADMAP/REQUIREMENTS — the commit follows directly)

## Files Created/Modified
- See key-files above; the evidence pair `18-08-auth-{a,b}.log` (550 lines each, identical marker positions 501/528/535 — the round-trip is deterministic in emission shape)

## Decisions Made
- See key-decisions: the one-pass-over-both-plans FOD discipline; lock-only never-bundled; the delivered-bytes verdict; the -s 127.0.0.1 venue pin; add-never-substitute on the flip

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The secstore client needed `-s 127.0.0.1` — the `$attr` cs translation class reds on r9**
- **Found during:** Task 2's first live gate run (the gate red at AUTH-FAIL secstore-roundtrip)
- **Issue:** The plan's client invocation (no `-s`) rides the client's default serve `$auth` → `tcp!$auth!5356`, which goes through cs's `$attr` translation; that path ENOETs inside cs's system-identity substrate on r9 — the kernel's `namec: does not exist` is relayed verbatim as the cs error (`devmnt: server error: cs: namec: does not exist` → `secstore: can't dial tcp!$auth!5356` in the first run's capture). The literal-IP dial class works on this venue every run (hauthprobe legs in the SAME capture).
- **Fix:** `-s 127.0.0.1` on both client invocations (secstore's own flag, secstore.c:528) — secstored's loopback listener pinned; the mechanism documented in the sweep's 9d comment and the README's venue-facts list.
- **Files modified:** build/x86_64/auth-sweep.rc (+ build/x86_64/README.md in Task 3's records pass)
- **Verification:** the next gate run green end-to-end (ROUNDTRIP-GREEN), then green twice
- **Commit:** 946da6635

### Verify adaptations (not deviations)
- The PIN-LOCK-ONLY check reads the commit-to-commit diff (`git diff HEAD~1 HEAD`) rather than the working-tree diff — the tree carries the pre-existing `sys/src/cmd/eqn/eqn.c` local scratch the dispatch forbids touching, which the working-tree form would count (the 18-07 precedent, same exclusion class).

**Total deviations:** 1 auto-fixed (Rule 3 blocking). **Impact:** the -s pin is a venue truth about r9's cs `$attr` class, now recorded in three places (sweep comment, README venue facts, STATE resolution) — it does not weaken the round-trip (the listener IS the loopback one; the hauthprobe legs use the same literal class).

## Issues Encountered
None new. (The x86_64 tick_check gap stays 18-07's recorded deferred item — adjacent latent defect, untouched here; the WINDOWS ledger remains broken from 18-05 (pre-existing frontmatter drift), and this plan opens no new window entries — no stubs, no skipped tests, no unrun verifies.)

## Authentication Gates
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 19's entry criterion now includes the secstore round-trip: loopback p9sk1 green pre-drawterm with a committed, re-runnable gate that is green twice at the pinned kernel (r9 9103522)
- Multi-write TCP (exportfs over cpu connections) rides the fixed seam; the bulk-write (>2048-byte) send-arm stall class is closed by the same ack_delay(None)
- The AUTH-02/03/08 shared-ID gate releases with this SUMMARY on disk; the remaining AUTH rows belong to the re-verification pass per the gap report's escalation note

## Self-Check: PASSED

All key files exist on disk (the evidence pair, this SUMMARY, r9's re-wired flake.nix, the sweep/expect/yml/README/REQUIREMENTS/STATE set); all three Harvey task commits verified as ancestors of HEAD (eaf4879fe, 946da6635, 2e641f03b); r9 9103522 verified on origin/main (pushed, unpushed empty); the ledger-measured commit count is 3 (dee637069..HEAD at SUMMARY time); both evidence captures carry the full marker set with the password provably absent; no eqn.c byte in any commit.

---
*Phase: 18-in-guest-p9sk1-auth-domain*
*Completed: 2026-11-14*
