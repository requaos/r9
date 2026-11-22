---
phase: 18-in-guest-p9sk1-auth-domain
plan: 05
subsystem: auth (p9sk1 probe legs, speaksfor C client, secstore substrate, CI leg, AUTH-09 docs)
tags: [p9sk1, speaksfor, authprobe, auth/debug, secstore, secuser, ramfs, mntgen, expect-gate, ci-leg, AUTH-06, AUTH-07, AUTH-08, AUTH-09]

requires:
  - phase: 18-in-guest-p9sk1-auth-domain
    provides: 18-04's auth image (§14 bake, listeners, cs, factotum, the /boot/authfeed.txt sentinel) + 18-03's generator semantics
provides:
  - "the loopback probe gate green twice (Phase 19's entry criterion, a committed re-runnable gate): auth/debug's full p9sk1 as norbert + the speaksfor wire form (hostid!=uid) proven with the fail-closed deny control + tr-ok on serial"
  - "build/x86_64/authprobe.c — the leg-2 client (debug.c's authfutz with the speaksfor delta; the self-speaks guard makes the SPEAKSFOR-OK marker unreachable from hostid==uid)"
  - "the secstore substrate: the ramfs writable venue (the cmd/ramfs tier leaf, mounted before the listeners, live file probe) + secuser's console-fed account creation"
  - "the AUTH-09 scoping docs: the README AUTH section + the PROJECT.md Key Decisions row (baked+paired for CI; host-9P /adm for real use — documented, not wired)"
  - "the auth CI leg in r9-shell.yml (R9_ACCEL=tcg; hosted observation PENDING FIRST PUSH)"
affects: [Phase 19 (drawterm dials the hostfwd-5670 seam onto the proven listener; the cpu path rides the speaksfor form), the next r9 kernel-byte pass (the devip two-write blocker's reopener)]

actuals:
  tokens: 78399   # chars/4 over the whole realized diff (806036850..8fce2f5f8: 313595 chars, 13 files, +9004/-14) — the retained evidence captures dominate (260289 chars); the authored code+docs diff is 53306 chars (13327 tokens). Estimate was 40000 — the authored-content miss is the usual over-projection family; the evidence retention is the plan's own design
  tasks: 3
  commits: 3      # measured: git rev-list --count 806036850..HEAD (2cd66969a, 897567d46, 8fce2f5f8)
plan_head_before: 8060368500250592b6bc5b2c251651c7ee749f13
plan_head_after: 8fce2f5f8

tech-stack:
  added: []        # no new packages: authprobe compiled by the in-tree 6c/6l with the #pragma-lib autolib record (T-18-SC kept)
  patterns:
    - "whitespace-tolerant gate patterns over captured in-guest echoes: rc's unquoted echo re-splitting cannot be relied on (the tab survived — the newline-only ifs STAYS for line-granular captures), so the expect pattern carries the class [ \\t]+"
    - "the named-SKIP gate arm (the 15-08 venue-honesty lineage extended): a venue-blocked leg emits AUTH-SKIP + AUTH-SKIP-WHY and the gate's post-pass holds the would-be verdict PROVABLY absent — no unearned OK, structurally"
    - "console-fed program prompts as the ONLY typed surface: getpassm (raw, no echo) and line-granular cooked reads; secuser's userinput whole-chunk reads coalesce any file feed, so /dev/cons IS the only stdin that answers per-prompt"

key-files:
  created:
    - build/x86_64/authprobe.c
    - build/x86_64/secstore-obj.txt
    - .github/workflows/r9-shell.yml (the auth-smoke-x64 job)
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-05-auth-a.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-05-auth-b.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-05-venue-mntgen-refusal.console.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-05-secstore-stall-modes.console.log
  modified:
    - build/x86_64/mkimage.sh        # 14i3 authprobe build; the tcp=ticket port=567 service row; 14f2b the host-side feed copy; 14f4 the payload object; the /adm/secstore mount point; ramfs in the staging set
    - build/x86_64/auth-sweep.rc     # legs 3b (venue), 7-8 (the probe window), 9 (secuser + the SKIP arm)
    - build/x86_64/smoke-r9-auth.expect  # the probe + secuser feed stages, the count/absent post-passes, the redaction
    - nix/smoke-r9-auth-x64.sh       # the R9_DEMO_PW export
    - build/x86_64/tiers/tier2-services.txt  # the cmd/ramfs leaf
    - build/x86_64/README.md         # the AUTH section
    - .planning/PROJECT.md           # the phase-close Key Decisions row

key-decisions:
  - "The venue verdict (the plan's Task-2 probe): RAMFS. mntgen's FILE writes are read-only on this tree (15-06's measurement) and the live re-confirmation measured a SECOND fact: rc's output-redirect open failure is FATAL to a sourced sweep (the first probe run's sweep rc died AT the probe line, `devmnt: directory opens OREAD only' — no AUTH-FAIL could even name the leg), so an in-sweep mntgen file probe cannot fail gracefully; the venue decision rides the recorded evidence and the LIVE in-gate probe proves the ramfs venue instead (create/write/read through MREPL|MCREATE, re-proven every gate run)"
  - "The venue mounts BEFORE the listeners (leg 3b): secstored's per-connection children snapshot the namespace at the listener's fork — mounting first makes the venue visible by construction regardless of pgrp-sharing semantics (rc's & forks WITHOUT RFNAMEG, so the shared-pgrp mount WOULD be visible — but the ordering removes the dependency on that semantics entirely)"
  - "secuser's field prompts are console-typed, not file-fed: userinput's whole-chunk read(0, buf, 128) over a file returns EVERY feed line at once (coalesced), so the plan's staged-feed mechanism is structurally broken for the 4 field prompts; the cooked console is the only line-granular stdin. The two getpassm passwords are console-fed as planned (R9_DEMO_PW — the driver reads the bake's host-side feed copy, mkimage 14f2b). /boot/secuser-feed.txt is therefore NOT staged (the staged-feed acceptance item is superseded by this measured mechanism — deviation 4)"
  - "T-18-13's redaction: auth/debug's noise leg (the cpu-server-owner prompt eats the sentinel line) puts the per-bake DEMO password into ONE authsrv -d tr-ok syslog line (authid logged verbatim). The gate runs every assert over the TRUE capture, then maps the password out of the retained capture (both green captures carry <demo-pw-redacted>; the password is provably absent)"
  - "The blocker's gate shape: the round-trip legs are NOT run (a wedging client would kill the sweep), the sweep emits AUTH-SKIP secstore-roundtrip + AUTH-SKIP-WHY r9-devip-two-write-tcp, and the gate's post-pass asserts AUTH-OK secstore-roundtrip stays ABSENT — the phase's own prohibition (no secstore green without bytes-back) enforced structurally"
  - "The tcp=ticket port=567 ndb row (Rule 3): cs's ipserv translates alpha service names via ndbgetvalue(db,\"tcp\",name,\"port\") — without the row every dial(netmkaddr(a,nil,\"ticket\")) reds with `can't translate service'; appended at the auth bake's compose only (D-08 minimality — the services image keeps its semantics)"

patterns-established:
  - "Pattern: r9-pinned-venue blockers get named-SKIP arms with structural teeth — the blocked verdict's ABSENCE is itself asserted, so nobody can quietly claim the win before the kernel fix lands"
  - "Pattern: console-feed stages (expect match-prompt-then-send) drive raw/cooked program prompts deterministically; the sentinel-bind covers only path-opened OREAD prompts (readcons), getpassm ORDRW needs the live console"

requirements-completed: [AUTH-06, AUTH-07, AUTH-08, AUTH-09]   # the plan frontmatter verbatim; the marking verdicts below govern REQUIREMENTS.md: AUTH-06/07/09 complete, AUTH-08 PARTIAL-BLOCKED (the substrate green, the round-trip venue-blocked)

coverage:
  - id: D1
    description: "auth/debug's full p9sk1 challenge-response as norbert from the boot flow (leg 1) — the sentinel-driven prompts, its OWN success line asserted"
    requirement: AUTH-06
    verification:
      - kind: e2e
        ref: "both evidence captures: `AUTH-DEBUGOBS <tab>ticket request using norbert@DEMO-harvey key succeeded' (the filtered echo — only the matching line reaches serial) + `AUTH-OK p9sk1-norbert'"
        status: pass
    human_judgment: false
  - id: D2
    description: "The speaksfor leg (leg 2): the hostid!=uid wire form via the purpose-built C probe, decrypted with the SPEAKER's key, cuid==suid==norbert asserted, the marker unreachable from a self-speaks form"
    requirement: AUTH-06
    verification:
      - kind: e2e
        ref: "both captures: `authprobe: SPEAKSFOR-OK bootes for norbert' + `AUTH-OK speaksfor'; the self-speaks guard at authprobe.c:120 and the t.num/cuid/suid assertions at :162-166"
        status: pass
      - kind: integration
        ref: "mkimage 14i3: the host-side 6c/6l build with the 00008a97 magic assert; the bake manifest asserts /amd64/bin/hauthprobe"
        status: pass
    human_judgment: false
  - id: D3
    description: "The negative control: norbert speaking for the denied uid sys — the scrambled reply fails to decrypt, exactly one attempt, the fail-closed path's server line asserted"
    requirement: AUTH-06
    verification:
      - kind: e2e
        ref: "both captures: exactly one `SPEAKSFOR-DENIED' (the gate's count_occurrences post-pass) + `AUTH-OK speaksfor-deny' + the tr-fail `no speaks for' syslog line present (Pitfall 11 armed)"
        status: pass
    human_judgment: false
  - id: D4
    description: "AUTH-07 closed: real tr-ok lines from the successful exchanges asserted on the gate's captures (alongside the pairing banner and ANNOUNCE)"
    requirement: AUTH-07
    verification:
      - kind: e2e
        ref: "both captures carry `tr-ok' (authsrv -d through the #c/cons fallback — the gate's post-pass); `authdom: PAIRED DEMO' + ANNOUNCE re-proven"
        status: pass
    human_judgment: false
  - id: D5
    description: "The writable /adm/secstore venue decided and proven live (ramfs; the mntgen refusal evidence retained)"
    requirement: AUTH-08
    verification:
      - kind: e2e
        ref: "both captures: `AUTH-VENUEOBS ramfs-file-write-ok' (create/write/read through the MREPL|MCREATE mount, mounted before the listeners); evidence/18-05-venue-mntgen-refusal.console.log carries the live mntgen refusal"
        status: pass
    human_judgment: false
  - id: D6
    description: "secuser creates norbert's secstore account through the venue (console-fed getpassm passwords + cooked-console field answers)"
    requirement: AUTH-08
    verification:
      - kind: e2e
        ref: "both captures: `change written' (after the six prompt feeds) + `AUTH-OK secuser' — who/, store/, and who/norbert standing through the mount (the diagnostic cycle's ls evidence)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The client PUT/GET round-trip with the bytes-back assert — BLOCKED on this venue (the named SKIP; see the blocker)"
    requirement: AUTH-08
    verification:
      - kind: integration
        ref: "the isolation matrix (evidence/18-05-secstore-stall-modes.console.log): the donor SConn two-write client pattern stalls with both procs parked (reproduced 3x across independent runs incl. the real client as the FIRST connection); a one-write control on the same listener round-trips PAKserver's framed error reply; the server's own two-write reply DELIVERS to a parked reader — the defect is r9's devip delivery/wake for the two-write client pattern"
        status: fail
    human_judgment: false
    rationale: "BLOCKED, not unproven: the payload object is staged (secstore-obj.txt with the fixed token), the account exists, the venue is writable — the exchange itself cannot complete on the pinned kernel. The gate holds AUTH-OK secstore-roundtrip PROVABLY absent until the devip fix lands (no unearned OK — the phase's own prohibition). AUTH-08 stays PARTIAL: the substrate (venue + account) is green, the round-trip is the open half."
  - id: D8
    description: "AUTH-09: the scoping documentation — the README AUTH section + the PROJECT.md Key Decisions row"
    requirement: AUTH-09
    verification:
      - kind: integration
        ref: "AUTH09-DOCS-LANDED: the README AUTH section covers the interface shape (authdomain.example / gitignored /authdomain.local / ephemeral loud-DEMO), the documented-not-wired host-9P verdict, the venue decision, the marker vocabulary, the blocker; the PROJECT.md row landed (the standard split)"
        status: pass
    human_judgment: false
  - id: D9
    description: "The full gate green twice consecutively on KVM with both captures retained + the CI leg wired at TCG"
    requirement: AUTH-06
    verification:
      - kind: e2e
        ref: "FULL-GATE-GREEN-TWICE: evidence/18-05-auth-{a,b}.log both carry AUTH-DONE, all eight AUTH-OK legs, tr-ok, SPEAKSFOR-OK, exactly one SPEAKSFOR-DENIED, the venue + secuser markers, the honest SKIP, zero AUTH-FAIL; CI-LEG-WIRED: the auth-smoke-x64 job with env R9_ACCEL=tcg nix run .#boot-r9-auth-x64, the sibling jobs' pinned-action structure (hosted observation PENDING FIRST PUSH per the standing convention)"
        status: pass
    human_judgment: false

duration: 105min
completed: 2026-11-14
status: complete
---

# Phase 18 Plan 05: Probe Legs + Secstore + CI + AUTH-09 Docs Summary

**Both loopback probe legs green as a committed re-runnable gate (auth/debug's own success line as norbert + the speaksfor hostid!=uid wire form with its fail-closed deny control, tr-ok on serial, green twice, CI-wired) — the secstore substrate green through the ramfs venue with secuser's console-fed account, while the client PUT/GET round-trip is NAMED-SKIPPED on a measured r9 devip defect (the two-write TCP pattern never delivers; kernel pinned, blocker recorded with structural gate teeth) — Phase 19's entry criterion is met on AUTH-06/07 with AUTH-08's open half loudly tracked.**

## Performance
- **Duration:** 105 min - **Started:** 2026-11-14T05:18Z - **Completed:** 2026-11-14T07:03Z - **Tasks:** 3/3 - **Files:** 13 (+9004/-14 over 806036850; the retained evidence captures dominate the byte count)

## Accomplishments
- AUTH-06 CLOSED: the speaksfor gate is proven with the wire form stock auth/debug cannot send — authprobe.c (host-side 6c/6l, the svctcpecho harness style) sends Ticketreq{hostid=bootes, uid=norbert}, decrypts with the SPEAKER's key, asserts cuid==suid==norbert; the self-speaks guard (speaker==user is a usage error) makes the marker structurally unreachable from the hostid==uid form. The deny control (uid=sys) fired exactly once per capture with the tr-fail `no speaks for' server line asserted — the fail-closed path armed
- AUTH-07 CLOSED: real tr-ok lines on serial in every capture (authsrv -d via the #c/cons fallback) — 18-04's flagged assumption discharged by this plan's real exchanges; the shared-ID hold released with this SUMMARY on disk
- The p9sk1 probe window runs from the boot flow: the sentinel bound over /dev/cons drives auth/debug's password prompts (the bootes-owner cooked-prompt noise accepted per A4, its password-bearing prompt text FILTERED out of the serial echo — only the matching success line echoes), and the '#c/cons' re-bind restores the live console for the secstore leg's getpassm (AUTH-CONSOLEOBS)
- The secstore SUBSTRATE is green: the ramfs writable venue (the 15-06-named alternative, live-probed every gate run) + secuser's account creation with console-fed passwords (the R9_DEMO_PW path: the bake copies the per-bake feed to gitignored out/, the driver exports it, the gate sends it raw — no echo) and cooked-console field answers
- AUTH-08's open half is honestly tracked: the client round-trip cannot complete on the pinned kernel (see the blocker); the gate asserts the round-trip verdict stays ABSENT — the phase's own prohibition (no secstore green without bytes-back) enforced structurally
- AUTH-09 landed: the README AUTH section + the PROJECT.md row; the CI leg joins r9-shell.yml at TCG
- Both evidence captures password-redacted (T-18-13): the asserts run over the true capture, then the DEMO password is mapped out — verified absent from both retained logs

## Task Commits
1. **Task 1: Probe legs — auth/debug full p9sk1 as norbert + the speaksfor C probe with negative control** - `2cd66969a` (feat; the gate green end-to-end, PROBE-LEGS-GREEN)
2. **Task 2: secstore substrate — venue + secuser green, the round-trip named-skipped on the measured devip blocker** - `897567d46` (feat; SECSSTORE-SUBSTRATE-GREEN-SKIP-ARMED)
3. **Task 3: green-twice full gate + CI leg + AUTH-09 docs** - `8fce2f5f8` (feat; FULL-GATE-GREEN-TWICE + CI-LEG-WIRED + AUTH09-DOCS-LANDED)

**Plan metadata:** this SUMMARY + STATE/ROADMAP/REQUIREMENTS (the docs commit follows)

## Files Created/Modified
- See key-files above; the evidence bundle grew four files (the two green captures + the mntgen-refusal + the stall-modes isolation matrix)

## Decisions Made
- See key-decisions: the ramfs venue (the fatal-redirect discovery), the pre-listener mount ordering, the console-typed field answers (userinput's read-coalescing), the T-18-13 redaction design, the blocker's structural gate shape, the tcp=ticket service row

## THE BLOCKER (r9 devip — AUTH-08's open half)
**r9's devip loopback TCP never delivers SConn's donor write pattern** — a 2-byte binary count write immediately followed by the payload write (`secstore/SConn.c` SC_write). Measured across three independent runs:
- the REAL client (two writes) stalls on its FIRST connection (two full gate runs + one probe run): both procs park forever — secstored's child in its readstr, the client in its readstr
- a ONE-WRITE control (the same 33 bytes concatenated) on the same listener round-trips: PAKserver reads, parses, refuses with a framed error reply — and the server's reply itself goes through the SAME two-write pattern on its side and DELIVERS to the parked client's read
- an alarm note never reaches a devip-blocked proc (secprobe's 15s alarm left it parked for minutes) — a second integrity symptom of the same blocking-wait cluster
- the defect is therefore the two-write CLIENT pattern class in r9's devip seam (the fast-path send never drives; the parked reader's drive never lands the pair) — KERNEL BYTES, pinned at b2a6151 == origin/main, outside this plan's and this repo's authority; the phase's FOD pass was consumed by 18-02
- **Reopener:** the next r9 kernel-byte pass — Phase 19's cpu path rides multi-write TCP (exportfs over cpu connections) and will hit this class immediately; unblocking = fix devip, restore the sweep's 9d legs (the committed shape: copy the payload into the venue, PUT with CWD there, -G GET, grep the payload line), flip the gate's absent-assert to presence demands. Recorded in STATE (add-blocker) + the README's blocker section + the gate's structural teeth
- The WINDOWS ledger append FAILED (pre-existing frontmatter/entry count drift in .planning/WINDOWS.md — 52/3/13 vs 49/5/14; the tool refuses and has no recount verb). The defect is instead recorded in STATE's blocker list, the README, and here — the ledger repair belongs to the planning-tooling owner

## Deviations from Plan

**1. [Rule 1 - Bug] My first sweep edit accidentally deleted leg 6 (the factotum substrate)**
- **Found during:** Task 1's first gate run (AUTH-FAIL immediately after the cs verdict)
- **Issue:** the Edit that appended legs 7-8 replaced leg 6's body instead of inserting after it — $factstat was never set, the probe window was skipped, the sweep red at its first new verdict
- **Fix:** leg 6 restored verbatim before leg 7; the gate re-ran green
- **Files modified:** build/x86_64/auth-sweep.rc
- **Verification:** AUTH-OK factotum + the full window in the next run's capture
- **Commit:** 2cd66969a

**2. [Rule 1 - Bug] The AUTH-DEBUGOBS echo kept auth/debug's leading tab — the gate's stage pattern (a literal space) never matched**
- **Found during:** Task 1's second gate run (the gate hung at the debug-success stage until timeout)
- **Issue:** the sweep's newline-ifs capture preserves the line's leading tab, and the save/restore of the default ifs was itself lossy (`ifsbackup=$ifs` goes through rc's assignment-time splitting — the saved copy came back EMPTY, leaving ifs='' for the rest of the file; the unquoted echo could not re-split)
- **Fix:** no save/restore — the newline-only ifs STAYS (every later backquote is line-granular by design; documented in the sweep), and the gate's pattern is whitespace-class tolerant (`AUTH-DEBUGOBS[ \t]+ticket request using norbert@`)
- **Files modified:** build/x86_64/auth-sweep.rc, build/x86_64/smoke-r9-auth.expect
- **Verification:** the gate green through the probe stages
- **Commit:** 2cd66969a

**3. [Rule 3 - Blocking] The tcp=ticket service row — dial-by-name needs an ndb row cs can translate**
- **Found during:** Task 1 design (verified against cs.c's ipserv before the first run)
- **Issue:** auth/debug's own dial form is netmkaddr(addr, nil, "ticket"); cs's ipserv translates alpha service names via ndbgetvalue(db,"tcp",name,"port") — without a `tcp=ticket port=567' row every probe dial reds with `can't translate service'
- **Fix:** the row appended at the auth bake's /lib/ndb/local compose (14f3) with both-way asserts — auth-image-only (the services image keeps its semantics, D-08 minimality)
- **Files modified:** build/x86_64/mkimage.sh
- **Verification:** every probe dial in both captures completed
- **Commit:** 2cd66969a

**4. [Rule 1 - Bug] secuser's staged-feed mechanism is structurally broken — userinput's reads coalesce a file feed**
- **Found during:** Task 2 design (read of secuser.c:210-227)
- **Issue:** userinput does whole-chunk `read(0, buf, sizeof buf)' — over a FILE stdin every feed line returns at once in one buffer, so the expires prompt gets all four answers coalesced (`bad date format') and the loop dies at EOF; a paced pipe is timing-fragile against the expect-typed password delay
- **Fix:** the field prompts read the COOKED CONSOLE (`< /dev/cons` — the line discipline delivers one line per prompt; the gate types the four pinned answers after matching each prompt). /boot/secuser-feed.txt is not staged (the staged-feed acceptance item is superseded by the measured mechanism); the two getpassm passwords are console-fed exactly as planned via R9_DEMO_PW (the mkimage 14f2b host-side feed copy + the driver export)
- **Files modified:** build/x86_64/auth-sweep.rc, build/x86_64/smoke-r9-auth.expect, nix/smoke-r9-auth-x64.sh, build/x86_64/mkimage.sh
- **Verification:** all six feeds landed in prompt order; `change written' + AUTH-OK secuser in both captures
- **Commit:** 897567d46

**5. [Rule 3 - Blocking] The mntgen venue probe cannot fail gracefully — rc's redirect-open failures are FATAL to a sourced sweep**
- **Found during:** Task 2's first gate run (the sweep rc died AT the probe line)
- **Issue:** the plan's probe shape (`hecho x > /adm/secstore/probefile' under a mntgen mount) fails with `devmnt: directory opens OREAD only' — and rc treats a failed output-redirect open as a script-fatal error: the sweep died before any AUTH-FAIL could name the leg
- **Fix:** the venue DECISION rides the recorded evidence (15-06's measurement + this live refusal, retained as evidence/18-05-venue-mntgen-refusal.console.log); the LIVE in-gate probe proves the ramfs venue instead (the cmd/ramfs tier leaf; create/write/read through the MREPL|MCREATE mount). Residual documented in the sweep: a ramfs refusal would kill the sweep at the probe line and the gate's venue stage localizes by timeout
- **Files modified:** build/x86_64/auth-sweep.rc, build/x86_64/tiers/tier2-services.txt, build/x86_64/mkimage.sh
- **Verification:** AUTH-VENUEOBS ramfs-file-write-ok in both captures
- **Commit:** 897567d46

**6. [Rule 3 - Blocking] The stale out/authfeed.txt refused the bake's copy (0444 generator output)**
- **Found during:** Task 2's second gate run (cp: Permission denied at 14f2b)
- **Issue:** the generator emits the feed read-only; the first run's stale copy blocked the re-run's overwrite
- **Fix:** rm -f before cp (the paqs' own rule)
- **Files modified:** build/x86_64/mkimage.sh
- **Verification:** the bake green on every later run
- **Commit:** 897567d46

**7. [BLOCKER - documented, not auto-fixed] The secstore client round-trip is venue-blocked (r9 devip, kernel pinned)**
- See THE BLOCKER above — beyond Rules 1-3 (the fix is kernel-side; r9 is pinned at b2a6151 and the phase's FOD pass is consumed), handled as a recorded blocker with the honest named-SKIP gate arm (the 15-08 venue-honesty lineage; the phase's own prohibition holds)
- **Commits:** 897567d46 (the SKIP arm), 8fce2f5f8 (green twice with the SKIP)

**Total deviations:** 7 (2 Rule-1 bugs, 3 Rule-3 blockers, 1 self-inflicted Rule-1 replaced-text bug, 1 recorded blocker). **Impact:** deviations 4-5 reshaped Task 2's mechanisms on measured rc/read semantics (both documented in the sweep's comments); deviation 7 is the plan's — and the phase's — one open edge, loudly tracked with its reopener.

## Issues Encountered
- Two background gate runs were reaped by the harness mid-post-pass (no output, no notification); the remedy was setsid-detached runs with status files. One reap left a qemu survivor holding 127.0.0.1:5670 — the 18-04-documented hostfwd-collision class, swept by tag
- The panic fast-fail and AUTH-FAIL fast-abort truncate the failing line mid-word (known); the leg name is still inferable from emission order
- The WINDOWS ledger append failed on pre-existing count drift (see THE BLOCKER's last bullet) — the defect is recorded in STATE/README/this SUMMARY instead
- The services venue stays blocked pre-existing (#68, STATE 17-06/17-07) — no services leg added, per the standing convention

## Authentication Gates
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- **Phase 19's entry criterion is met on its AUTH-06/07 substance**: the loopback p9sk1 probe is a committed re-runnable gate, green twice, CI-wired; drawterm dials the hostfwd-5670 seam onto the proven listener (authsrv -d via listen1, tr-ok on serial); the cpu server's speaks-for form is proven live
- **AUTH-08's open half follows the devip fix**: Phase 19's cpu path rides multi-write TCP (exportfs over cpu connections) and will hit the same class — the fix should land in the Phase-19 r9 kernel pass, then the sweep's 9d restore + the gate's absent-assert flip close AUTH-08 (the payload, the account machinery, and the venue are all in place and green)
- The 112-byte Nvrsafe fact, the tcp=ticket row, and the venue ordering are recorded for every future auth-image consumer
- The AUTH-09 scoping is documented-not-wired as locked (D-15): the host-9P /adm mount belongs to the phase that first needs a real domain

## Self-Check: PASSED

All key files exist on disk (authprobe.c, secstore-obj.txt, the modified mkimage/sweep/expect/driver/tier/README/PROJECT, all six evidence files, this SUMMARY); all three task commits verified as ancestors of HEAD (2cd66969a, 897567d46, 8fce2f5f8); ../r9 untouched at b2a6151 == origin/main; eqn.c and the other pre-existing dirty files untouched by any commit; both retained captures carry the full marker set with zero AUTH-FAIL and the password provably redacted.
