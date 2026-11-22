---
phase: 15-userspace-tiers-bootfs-payload-amd64
plan: 01
subsystem: tier-machinery
tags: [tiers, tier2, services, subset-build, mk-filters, aux-leaf-filter, anti-staleness, ci, uspace-07, x86_64]
duration: 30 min
completed: 2026-11-09
requires:
  - "14-08-complete tier machinery (the allowlist data + five mk filter arms + the subset driver + T1-ART markers this plan appends onto)"
provides:
  - "build/x86_64/tiers/tier2-services.txt — the append-only services-tier allowlist (65 entries: ndb, exportfs, dial, mntgen, srv, import, aux:listen, 19 ip singles + 6 ip dirs, 32 auth singles + secstore) with per-class provenance and D-10 named exclusions"
  - "the six mk filter sites reading the tier1+tier2 UNION in one multi-file awk — including the NEW aux leaf filter (^cmd/aux:-anchored TARG keep + ^cmd/aux/ DIRS keep), Pitfall 3 closed in the same commit as the first cmd/aux: entry"
  - "subset-build.sh's tier-2 loop with the 06-03 anti-staleness decision LANDED (per-entry fresh build: mk <t>.clean / in-tree rm before every build) + T2-ART sha256 markers + 00008a97 magic checks + the phase build-tree lock (flock .build/amd64-tree.lock, held for the whole run — the declared coupling point for 15-02/15-05/15-07)"
  - "the scoping proof (evidence/15-01-scoping-proof.sh): the default objtype=amd64 build surface is EXACTLY tier1∪tier2 (mk-evaluated TARG/DIRS sets == the data's awk keep-rule expectations at cmd + all four container leaves; COVERAGE_SWEEP=1 bypasses every filter)"
  - "the tier-2 compile CI leg in phase6.yml (host-only, marker-parity-gated) with the venue-pin tripwire scoped to the aarch64 donor scripts"
affects:
  - "build/x86_64/tiers/{tier2-services,tier0-libs}.txt, build/x86_64/subset-build.sh, sys/src/{mkfile,cmd/mkfile,cmd/{ip,auth,aux}/mkfile,cmd/auth/factotum/mkfile}, .github/workflows/phase6.yml"
  - "15-02 (the donor services venue consumes the same tier data + lock), 15-03 (the r9 leg + the sweep's computed expectations), 15-04 (tier-3 rides the widened awk pattern), 15-07 (the coverage proof rides the COVERAGE_SWEEP escapes)"
tech-stack:
  added: []
  patterns:
    - "the filter-emptied container mkfile needs its variable-rule targets on the UNFILTERED list: a `$DIRS:V:` target line that expands empty is a mk parse error (\"no var on left side\") — retarget $ALLDIRS when the rule body iterates the filtered set anyway (the aux class, new this plan)"
    - "dir-entry driver artifacts located by mtime-stamp diff against /amd64/bin: member mkfiles legitimately disagree about BIN (ndb/dial→bin/<dir>, ppp/dhcpd/imap4d→bin/ip, ftpfs/snoopy/exportfs→bin root, httpd→bin/ip/httpd) — never assume the rel-path mapping"
    - "6l autolib chains propagate THROUGH archive members: linking anything against libhttpd needs libbin because libhttpd's own objects carry the <bin.h> autolib — closure appends must follow the transitive chain, not just the direct includes"
    - "plan9port tr does not process \\n escapes — `tr '\\n' ' '` eats every literal 'n' in the input; use awk joins in any script that may run with plan9 tools on PATH (and the devshell puts hostbin FIRST)"
key-files:
  created:
    - build/x86_64/tiers/tier2-services.txt
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-01-scoping-proof.sh
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-01-scoping-proof.log
  modified:
    - build/x86_64/subset-build.sh
    - build/x86_64/tiers/tier0-libs.txt
    - sys/src/mkfile
    - sys/src/cmd/mkfile
    - sys/src/cmd/ip/mkfile
    - sys/src/cmd/auth/mkfile
    - sys/src/cmd/auth/factotum/mkfile
    - sys/src/cmd/aux/mkfile
    - .github/workflows/phase6.yml
key-decisions:
  - "The 06-03 anti-staleness decision landed as per-entry fresh build IN the tier-2 driver loop (mk <target>.clean / the in-tree object rm before every build — deterministic, tier-local, in-driver), plus run-granularity freshness for the one shared in-tree archive (auth's lib.6.a removed at run end so every run rebuilds it): stale objects can never earn T2-ART markers"
  - "The container-leaf dir:target form (cmd/aux:<target>) dispatches link-only (mk 6.<target> + driver cp harvest to /amd64/bin/<target>) — aux's aggregate install never installs TARG members and the driver owns the destination; the aux leaf filter + the first cmd/aux: entry landed in the SAME commit (Pitfall 3)"
  - "gping re-derived OUT of tier-2 as the GUI class (links libdraw — measured live at the first run's 6l): a graphical tool, not a service; joins tier 3's libdraw closure, the factotum:fgui precedent — D-10's named-exclusion mechanism, never a quiet trim"
  - "The tier-0 closure grew exactly where the gate caught it: libdisk (iscsisrv), libhttpd (the httpd family), libbin (libhttpd's transitive archive-member autolib) — append-only with catching-provenance, the 14-08 discipline"
  - "The venue-pin tripwire in phase6.yml scoped to build/aarch64/subset-*.sh — the prohibition's intent (no 9front donor gates in CI) is unchanged and the x64 tier driver is the tier-2 leg's own tool"
requirements-completed: []   # USPACE-07 shared with 15-02/15-03/15-08 — ready only when every declaring plan has its SUMMARY (the compile half this plan delivered is recorded here)
actuals:
  tokens: 10700   # chars/4 over the realized diff (42780 chars, 12 files, 598 insertions — estimate was 60000: again a large over-projection, the same direction as the 14-08 analog)
  tasks: 3
  commits: 3      # measured: git rev-list --count 5dca05994..HEAD (the docs commit follows)
status: complete
coverage:
  - id: D1
    description: "Task 1 (tracer) — the tier-2 data file, the aux leaf filter, the six-site union awk, the T2 driver loop with per-entry fresh builds and the build-tree lock, proven end-to-end by the listen tracer"
    requirement: USPACE-07
    verification:
      - kind: command
        ref: "nix develop -c sh build/x86_64/subset-build.sh: exit 0, 'T2-ART listen <sha256>' + TIER-BUILD-OK in the log, amd64/bin/listen magic 00008a97 at the checkout path; grep tier2-services = 1/1/1/1/1 across the five sites + the aux leaf (cmd/aux: x2)"
        status: pass
      - kind: command
        ref: "tracer feedback gate (end-of-phase mode): both <automated> verifies re-run post-commit — VERIFY1-PASS + tier2-wiring-ok"
        status: pass
    human_judgment: false
  - id: D2
    description: "Task 2 — the full 65-entry services membership compiles green with 91 T2-ART markers, closure appends recorded, aarch64 + tier-1 data byte-frozen"
    requirement: USPACE-07
    verification:
      - kind: command
        ref: "driver exit 0; entries=65 markers=91 (M>=N), TIER-BUILD-OK, zero 6c/6l errors in the run log; git diff HEAD -- sys/src/ clean after (droppings), build/aarch64/ + tier1-cmds.txt unchanged; tier0 growth = 3 append-only libdisk/libhttpd/libbin lines with '# 15-01 append:' provenance"
        status: pass
    human_judgment: false
  - id: D3
    description: "Task 3 — the executable scoping proof (default amd64 build == exactly tier1∪tier2; COVERAGE_SWEEP escapes live) + the tier-2 compile CI leg"
    requirement: USPACE-07
    verification:
      - kind: command
        ref: "nix develop -c sh evidence/15-01-scoping-proof.sh: SCOPING-PROOF-OK — 8/8 PROOF-PASS (cmd-TARG, cmd-DIRS, aux-TARG={listen}, ip-TARG=19, auth-TARG=32, fact-TARG={factotum}, aux sweep 38==ALLTARG, fact sweep factotum+fgui); log committed at evidence/15-01-scoping-proof.log"
        status: pass
      - kind: command
        ref: "no bare 'cmd/aux' data line (awk check); grep tier2|tier-2|services phase6.yml = 14; COVERAGE_SWEEP present at all six filter sites; yaml parses"
        status: pass
    human_judgment: false
---

# Phase 15 Plan 1: Userspace Tiers + Bootfs Payload (amd64) — Tier-2 Services Compile Summary

**The tier-2 services membership as committed append-only data (65 entries → 91 T2-ART markers): the six mk filter sites reading the tier1+tier2 union in one multi-file awk incl. the NEW aux leaf filter, the subset driver's tier-2 loop with the 06-03 anti-staleness decision landed (per-entry fresh builds), the executable scoping proof that the default amd64 build carries exactly tier1∪tier2, and the tier-2 compile CI leg.**

## Performance

- **Duration:** 30 min
- **Started:** 2026-11-09T21:01:44Z
- **Completed:** 2026-11-09T21:31:56Z
- **Tasks:** 3/3 (Task 1 tracer)
- **Files modified:** 12 (3 created)

## Accomplishments

- **The tracer (Task 1)** proved the whole machinery on one entry before expansion: `cmd/aux:listen` rode data → six filters (incl. the new aux leaf) → namespace build → `/amd64/bin/listen` → `T2-ART listen <sha256>`, with the per-entry fresh-build rule live and the phase build-tree lock (flock `.build/amd64-tree.lock`) held for the whole run — the declared coupling point every later /amd64 writer (15-02 venue scripts, 15-05 build-9kdraw, 15-07 coverage-sweep) takes.
- **The full membership (Task 2)**: ndb (whole dir — cs/dns + the csquery/dnsquery probe clients), exportfs, dial (pass/expect), mntgen, srv, import, the ip suite (19 singles + dhcpd/ftpfs/httpd/ppp/imap4d/snoopy), the auth suite (32 singles + secstore; factotum never moved from tier-1), aux:listen — 65 entries, 91 markers, zero tool errors, sys/src git-clean after every run.
- **The scoping proof (Task 3)** is an executable committed script: mk dry runs inside the namespace extract the mk-EXPANDED TARG/DIRS sets and compare them against the tier data's own awk keep-rule expectations — 8/8 checks pass, including the Pitfall-3 converse (aux contributes ONLY listen under amd64; its full 38 members under COVERAGE_SWEEP=1).
- **The tier-2 compile CI leg** (phase6.yml): host-only subset-build with computed marker-parity gating (`T2-ART count >= tier data entries`), no execution legs per the Phase-14 venue precedent.

## Task Commits

1. **Task 1: the tier-2 tracer** — `c88d05b8e` (feat)
2. **Task 2: the full services membership** — `f10496775` (feat)
3. **Task 3: scoping proof + CI leg** — `0b394295e` (feat)

## The scoping proof (recorded per the plan's Task-3 action)

Command: `nix develop -c sh .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-01-scoping-proof.sh`
(the script generates its own namespace scaffold; the full log is committed beside it)

```
PROOF-PASS MK:cmd-TARG          (awk basename cat cmp comm date dc du echo fmt import join ls md5sum ...)
PROOF-PASS MK:cmd-DIRS          cc + (auth aux awk dial exportfs grep ip ndb paqfs rc)
PROOF-PASS MK:aux-TARG          listen                      <- the Pitfall-3 converse: aux's other 37 NOT built
PROOF-PASS MK:ip-TARG           6in4 dhcpclient ... wol (19 — ping/udpecho/gping excluded by name)
PROOF-PASS MK:auth-TARG         as asn12dsa ... wrkey (32)
PROOF-PASS MK:fact-TARG         factotum
PROOF-PASS MK:aux-TARG-SWEEP-COUNT (38 == aux ALLTARG's 38 — COVERAGE_SWEEP=1 bypasses the aux leaf filter)
PROOF-PASS MK:fact-TARG-SWEEP   factotum fgui
SCOPING-PROOF-OK
```

## Decisions Made

See key-decisions in the frontmatter — the load-bearing ones: the anti-staleness mechanism (per-entry clean, in-driver), the container-leaf link-only dispatch, gping's D-10 re-derivation out (GUI class), the transitive closure appends, and the venue-pin scoping.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A filter-emptied `$DIRS:V:` rule is a mk parse error**
- **Found during:** Task 1 (the first tracer run)
- **Issue:** aux's per-dir rule target `$DIRS:V:` expands to `:V:` under the amd64 filter (empty DIRS) — mk aborts "no var on left side of assignment/rule" before any target runs.
- **Fix:** The rule's target list moved to `$ALLDIRS` (always non-empty); the body iterates the filtered `$DIRS` either way, so behavior is identical whenever `$DIRS` is non-empty. Recorded as a pattern — every future filter-equipped container mkfile has this class.
- **Files modified:** sys/src/cmd/aux/mkfile
- **Verification:** the tracer build green; the scoping proof's aux evaluations parse and evaluate.
- **Committed in:** c88d05b8e

**2. [Rule 3 - Blocking verify] Task-1's `file /amd64/bin/listen | grep 'Plan 9'` is unusable on this host**
- **Found during:** Task 1 verify
- **Issue:** (a) `/amd64/...` exists only inside the namespace — the host-side equivalent is the checkout's `amd64/` path; (b) this host's file(1) reports `data` for Plan 9 a.out (no "Plan 9" magic entry), so the grep can never pass even in-namespace.
- **Fix:** The artifact check adapted to the checkout path + the `od -An -tx1 -N4` magic assert (00008a97) — the same check the tier-1 host-side spot-checks use, now also asserted for listen inside the driver itself. The driver must run inside the nix devshell (`nix develop -c`) — its own header documents this ($PLAN9 for the userns scaffold's rc bind).
- **Verification:** exit 0 with the spot-check live in the driver.
- **Committed in:** c88d05b8e

**3. [Rule 1 - Membership] gping re-derived out of tier-2 (GUI class)**
- **Found during:** Task 2 (the first full run's 6l)
- **Issue:** gping links libdraw — the out-of-tier graphics stack tier 3 owns.
- **Fix:** D-10 named exclusion in the tier-2 data (the factotum:fgui precedent), NOT a trim: the exclusion comment records the measured reason and the tier-3 destination.
- **Files modified:** build/x86_64/tiers/tier2-services.txt
- **Verification:** the ip filter's keep set = the 19 named singles (scoping proof).
- **Committed in:** f10496775

**4. [Rule 2 - Missing critical] The tier-0 closure needed three appends (one transitive)**
- **Found during:** Task 2 runs 2-4
- **Issue:** iscsisrv → libdisk; the httpd family → libhttpd; and libhttpd's own archive members carry the `<bin.h>` autolib → libbin at every consumer's link (the transitive class direct includes don't see).
- **Fix:** Append-only tier0-libs.txt growth with catching-provenance comments — exactly the plan's flagged-assumption path ("append-only growth ... never a scope trim").
- **Files modified:** build/x86_64/tiers/tier0-libs.txt
- **Verification:** 91/91 markers, zero 6l errors.
- **Committed in:** f10496775

**5. [Rule 3 - Blocking] The venue-pin tripwire matched the x64 driver's name**
- **Found during:** Task 3 (CI leg design)
- **Issue:** phase6.yml's `subset-(build|exec)\.sh` prohibition (written when those were the aarch64 9front donor gates) would red the whole workflow on the tier-2 leg's own `build/x86_64/subset-build.sh` reference.
- **Fix:** The pattern scoped to `build/aarch64/subset-(build|exec)\.sh` — the intent (no 9front donor gates in CI) unchanged; the pattern still never matches its own comment text.
- **Files modified:** .github/workflows/phase6.yml
- **Verification:** the greps re-checked; yaml parses.
- **Committed in:** 0b394295e

---

**Total deviations:** 5 auto-fixed (2 bugs, 1 missing-critical closure, 2 blocking/verify-environment). **Impact on plan:** all fixes are within the plan's own disciplines (D-10 named exclusions, append-only closure growth, marker-not-exit gating); no scope creep — the terminal truths (tracer end-to-end, 65→91 parity, scoping proof 8/8) are met exactly as planned.

## Authentication Gates

None.

## Known Stubs

None — no placeholder paths, no unwired data, no skipped verifications. The tier-2 data's named exclusions (gping, ipconfig, ping/udpecho carriers) are committed D-10 records, not gaps.

## Issues Encountered

- The devshell puts Harvey's plan9 tools FIRST on PATH — plan9port `tr` does not process `\n` escapes (eating every literal `n` in piped text) and plan9 `sed`/`ls` differ; the scoping-proof script pins absolute system-tool paths and awk joins (pattern recorded). Caught by proof-script output mangling, not by any build.

## Next Plan Readiness

READY for 15-02 (the donor services venue): the tier data, the filter scoping, the driver's per-entry fresh builds, and the build-tree lock it must take are all landed and proven. 15-03's computed expectations read the same tier2-services.txt. The ipconfig T1-SKIP flip (the 14-09 stale reason) is 15-02/15-03's sweep work, per plan.

## Self-Check: PASSED

- files exist: tier2-services.txt, evidence/15-01-scoping-proof.{sh,log}, subset-build.sh, tier0-libs.txt, the six mkfiles, phase6.yml ✓
- commits are ancestors of HEAD: c88d05b8e, f10496775, 0b394295e ✓
- plan-level verification green at HEAD: fresh full run exit 0, entries=65 markers=91, TIER-BUILD-OK, tracer marker, zero tool errors, sys/src clean, aarch64 frozen, listen magic 00008a97 ✓

---
*Phase: 15-userspace-tiers-bootfs-payload-amd64*
*Completed: 2026-11-09*
