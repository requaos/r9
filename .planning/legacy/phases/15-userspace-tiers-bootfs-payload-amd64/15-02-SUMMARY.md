---
phase: 15-userspace-tiers-bootfs-payload-amd64
plan: 02
subsystem: services-venue
tags: [services, functional-probes, svc-markers, k10t1, donor-venue, d-02, d-08, loopback, listen, exportfs, factotum, keyfs, authsrv, uspace-07, x86_64]
duration: 179 min
completed: 2026-11-10
requires:
  - "15-01-complete tier-2 compile machinery (tier2-services.txt membership + the T2-ART binaries + the phase build-tree lock this plan's venue scripts take)"
provides:
  - "build/x86_64/services-sweep.rc — the D-02/D-08 functional-probe sweep (BOTH venues' single source; 15-03's image bakes it unchanged): mntgen's walk-create bootstrap, cs's ipinfo answer, dns's baked A record, exportfs's content-through-mount, listen's loopback connect round-trip, factotum's ctl round-trip, keyfs's write-back-on-create, authsrv's connect-accept — bounded spins with SVC-FAIL arms, declared conditional SVC-SKIPs, SVC-DONE"
  - "build/x86_64/services-termrc — the services image's termrc override staged for 15-03's bake (the tier-termrc shape)"
  - "the grown k10t1 donor embed (58 files: +services-sweep.rc, +13 h*/tcpNNNN members incl. the staged tcp9901=echo and tcp9902=authsrv service programs) + k10t1.rc's services leg (child-rc source after the tier-1 sweep)"
  - "build/mm/boot-9k10t1-sweep.sh's services classification arm: computed SVC expectations from tier2-services.txt (+factotum's tier-1 line), SKIP set from the sweep's own echo lines, boundary-anchored + converse, zero SVC-FAIL, SVC-DONE poll target; the donor leg green 3x consecutively (T1-VENUE-PASS svc=9/0 ok/skip fail=0)"
  - "the donor venue's shared-tree discipline: build-9k10t1.sh + boot-9k10t1-sweep.sh both hold flock .build/amd64-tree.lock (the 15-01 coupling contract vs 15-04's same-wave builds)"
  - "build/x86_64/svctcpecho.c — the compiled one-shot loopback tcp client (read-a-line + connect-only modes), host-side 6c/6l (the nm-input.6 discipline) — the listen/authsrv probes' client"
  - "sys/src/9k/port/devroot.c: the standard /rc root dir (srv -e's exec'd rc needs /rc/lib/rcmain on every rc invocation — measured)"
affects:
  - "build/x86_64/{services-sweep.rc,services-termrc,svctcpecho.c}, build/mm/{build-9k10t1.sh,boot-9k10t1-sweep.sh}, sys/src/9k/k10/k10t1, sys/src/9k/k10/root/k10t1.rc, sys/src/9k/port/devroot.c"
  - "15-03 (the r9 services leg bakes this sweep + termrc unchanged; its expectations compute from the same tier data; the ipconfig T1-SKIP flip rides the r9 leg), 15-06 (mntgen scratch precedent), Phase 18 (the auth substrate: the D-08 boundary held — no p9sk1 anywhere)"
tech-stack:
  added: []
  patterns:
    - "Harvey rc's in-guest dialect is a measurement surface: 'if not' takes a SIMPLE command only (a braced 'if not {' body is a parse error that kills the whole SOURCED file mid-stream); a bare '=' inside an unquoted match pattern is a lexer token error; rc parses each braced block whole before running it — multi-step ladders use flat status-variable conditions ('if(~ $var state)') instead of nested if/if-not chains"
    - "the fd-dup redirect '<[0=7]' is unreliable inside backquotes in nested sourced blocks (measured: the dup landed reversed — a cat read the CONSOLE and wrote into the ctl channel, a silent block only serial input unmasks); the /fd dup-device read and the 14-09 fixture's transient clone-read + per-write ctl opens are the reliable shapes"
    - "libndb's Brdline parser drops a final unterminated line WHOLE — every generated ndb/test file carries its trailing newline; cs queries: an empty service part fails ipserv and the dom-attr network translation takes dnsiplookup first whose /srv/dns failure errstr swamps the ndb fallback — the general '!ipinfo dom=host attr' query is the measured ndb-backed route"
    - "libip's myipaddr skips 127.0.0.0/8 and never falls back to loopback — a loopback-only venue carries a second non-loopback probe address on the same ipifc for any daemon that needs 'my ip' (dns)"
    - "authcmdlib's getauthkey escalates failed readnvram into an INTERACTIVE console prompt (readln opens /dev/cons BY PATH) — a no-input venue isolates it the namespace way: a READ-ONLY file bound over /dev/cons feeds the same fixed line to password and confirm (an rw target lets the differing prompts rewrite the input — mismatch loop)"
    - "keyfs's server forks RFENVG — the keyfile its writeusers writes lands in the child's private env copy (unobservable in the parent); the through-mount user directory is the honest write-back observable (Create itself calls writeusers)"
    - "an interactive serial session (qemu -serial stdio with scripted input) is the fastest wedge-bisector on a serial-only venue — it unmasks silent blocks (a blocked stdin read consumes the typed line) and separates parse deaths from kernel wedges in one boot"
key-files:
  created:
    - build/x86_64/services-sweep.rc
    - build/x86_64/services-termrc
    - build/x86_64/svctcpecho.c
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-02-donor-services-serial.log
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-02-donor-services-harness.log
  modified:
    - sys/src/9k/k10/k10t1
    - sys/src/9k/k10/root/k10t1.rc
    - sys/src/9k/port/devroot.c
    - build/mm/build-9k10t1.sh
    - build/mm/boot-9k10t1-sweep.sh
key-decisions:
  - "The cs probe rides the general '!ipinfo dom=probehost ip' query: the tcp!host!port network translation takes dnsiplookup first (no /srv/dns on the venue) whose failure errstr swamps the successful ndb fallback — the ipinfo path answers straight from the baked ndb and is still a real client interaction through /net/cs"
  - "The loopback bring-up (the 14-09 fixture's own transient shape: clone read + per-write ctl opens, NO fd holds) precedes every daemon: cs registers its network list by scanning /net/*/clone at startup (measured: without #I bound, 'tcp' is an unknown network and queries fall through to the no-translation clone reply)"
  - "listen's per-connection programs are staged embed COPIES under their service names (tcp9901=echo, tcp9902=authsrv — listen.c execes servdir/<proto><port>): one hlisten -q -t /boot tcp announces both; the echo service's sent-line round-trip proves listen, the connect-accept proves authsrv (D-08's listening-proven shape — the p9sk1 handshake stays Phase 18's)"
  - "devroot gained the standard /rc root dir (NOT an Nbootfiles raise — 58 embedded files fit the existing 64 ceiling; A6 did not trigger): srv -e's exec'd rc needs /rc/lib/rcmain on every invocation, and the sweep bootstraps it namespace-style (mntgen /rc + bind -b /boot /rc/lib)"
  - "hmkdir and hsvctcpecho are host-side 6c/6l link-only builds in build-9k10t1.sh's staging (the nm-input.6 discipline: relative -I paths from the repo root, 6l -L to the checkout's amd64/lib) — mkdir is the write-flow class (not a tier member on x64) and the one-shot tcp client joins the netblast/tcpblast compiled-helper precedent"
  - "The donor capture is the evidence of record (evidence/15-02-donor-services-serial.log): every SVC-OK carries its OBS line on the capture — SVC-CSOBS ip=127.0.0.1, SVC-DNSOBS probehost.test, SVC-LISTENOBS tcp9901, SVC-FACTOBS the masked key, SVC-AUTHOBS connected"
requirements-completed: []   # USPACE-07 shared with 15-01/15-03/15-08 — ready only when every declaring plan has its SUMMARY (the donor-execution half this plan delivered is recorded here)
actuals:
  tokens: 11166   # chars/4 over the realized diff (41011) + evidence captures (3655); estimate was 58000 — the large over-projection continues the 15-01 pattern
  tasks: 3
  commits: 25     # measured: git rev-list --count 01e6e0e7f..HEAD (the docs commit follows)
commits: 25
plan_head_before: 01e6e0e7f5fa9892d1bab4f44690858ba08164f5
plan_head_after: 5fb06b0070171f24a2ae7a0462cb6d619635964d
status: complete
coverage:
  - id: D1
    description: "Task 1 (tracer) — mntgen + cs end-to-end through the grown donor embed, the boot-flow services leg, the SVC classification arm, and the image-facing services-termrc"
    requirement: USPACE-07
    verification:
      - kind: command
        ref: "nix develop -c sh build/mm/boot-9k10t1-sweep.sh: exit 0 — T1-VENUE-PASS svc=2/0 ok/skip fail=0, SVC-OK mntgen + SVC-OK cs + SVC-DONE, tier-1 leg ok=36 green in the same capture (regression); tracer feedback gate: verify re-run end-to-end post-commit"
        status: pass
      - kind: command
        ref: "acceptance: test -s both new files + grep services-sweep in services-termrc AND k10t1.rc; embedded root files: 47 (then 54, 58 as the tasks grew); grep tier2-services boot-9k10t1-sweep.sh = 2 (baseline 0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Task 2 — dns (baked A record), exportfs+srv (content through the guest-side mount), listen (loopback connect round-trip) with computed expectations and SKIP-set equality"
    requirement: USPACE-07
    verification:
      - kind: command
        ref: "exit 0, T1-VENUE-PASS svc=6/0 ok/skip fail=0, zero panic/trap, declared=7 SKIP lines >= emitted=0 (converse clean); SVC-DNSOBS/LISTENOBS answer lines on the capture"
        status: pass
    human_judgment: false
  - id: D3
    description: "Task 3 — the D-08 auth shallow subset (factotum ctl round-trip, keyfs write-back-on-create, authsrv connect-accept) + the full donor leg green"
    requirement: USPACE-07
    verification:
      - kind: command
        ref: "exit 0 — T1-VENUE-PASS svc=9/0 ok/skip fail=0 THREE consecutive full-leg runs (boot21/donorsvc3/bootfinal); all nine subjects exactly once as SVC-OK; SVC-FACTOBS 'key proto=pass service=test user=probe !password?' (the masked readback)"
        status: pass
      - kind: command
        ref: "the test-key boundary: grep -ci 'p9sk1|authkey' capture = 0 (zero real-credential artifacts); probe labeling greppable (probehost/probeuser/proto=pass); capture retained at evidence/15-02-donor-services-serial.log"
        status: pass
    human_judgment: false
---

# Phase 15 Plan 2: Userspace Tiers + Bootfs Payload (amd64) — Donor Services Venue Summary

**The k10t1 donor venue runs the committed services-sweep from the boot flow and all nine probe subjects earn interaction-backed SVC-OK markers — mntgen's walk-create, cs's ipinfo answer, dns's baked A record, exportfs's sentinel-through-mount, listen's loopback round-trip, factotum's masked-key ctl readback, keyfs's write-back-on-create, authsrv's connect-accept — with the tier-1 leg regression-green in the same capture and the sweep file ready for 15-03's r9 bake unchanged.**

## Performance

- **Duration:** 179 min (21:35Z→00:34Z; ~13 diagnostic boot cycles inside — the rc-dialect and daemon-startup findings below)
- **Started:** 2026-11-09T21:35:16Z
- **Completed:** 2026-11-10T00:34:43Z
- **Tasks:** 3/3 (Task 1 tracer)
- **Files modified:** 8 (+2 evidence captures)

## Accomplishments

- **The tracer (Task 1)** proved the donor vertical on two probes before expansion: the grown embed (the tier-2 binaries staged h*-prefixed), k10t1.rc's services leg, the harness's computed-expectation SVC arm, and the image-facing services-termrc — then Tasks 2/3 appended the remaining seven probes onto the proven slice.
- **Every marker is interaction-backed (D-02's letter):** cs answered `ip=127.0.0.1` from the baked ndb, dns resolved `probehost.test`, exportfs served the tier-1 sentinel byte-compared THROUGH the srv -e mount, listen's announced port accepted a real connect whose per-connection service (the staged tcp9901=echo) sent its line back, factotum read back the installed test key with the secret masked, keyfs's DMDIR create ran the write-back (the user dir stands through the mount), authsrv's announce accepted the connect (D-08's listening-proven — the handshake stays Phase 18's).
- **The venue grew real infrastructure:** the standard /rc root dir in devroot + the /rc/lib bootstrap (mntgen + bind) that makes ANY srv -e service runnable on the venue; the two-address loopback ipifc (127.0.0.1 + a non-loopback probe address for myipaddr-dependent daemons); the compiled one-shot tcp client; the host-side link-only build lane in the staging phase.
- **The classification arm has teeth:** computed expectations verify the probe-subject entries against the tier data (factotum verified against its tier-1 line), the SKIP set extracts verbatim from the sweep's own declared echo lines with the converse assertion, boundary-anchored regexes, zero-SVC-FAIL, and SVC-DONE as the phase-B poll target — green 3x consecutively.

## Task Commits

1. **Task 1 (tracer): mntgen + cs end-to-end** — `64506863b` (feat) + fixes `1a43d5b16` `5ecff5596` `cf2149c59` `e636ce9f6` `3c7547765` `1e1a8aeef` `db34e2aea` `1af91fe7a` `4fc9764e1` `7ebe2c53f` (the rc-dialect and probe-shape findings, each measured live)
2. **Task 2: dns + exportfs/srv + listen** — `ebf06c533` (feat) + fixes `4e870fcf8` `d423eb4ad` (myipaddr + the /rc root dir)
3. **Task 3: the D-08 auth subset** — `2c5be2b4c` (feat) + fixes `aa0e35fdf` `1998e9ccf` `42c96e231` `5fb06b007` (the keyfs prompt isolation + observable)

**Plan metadata:** (docs — follows)

## Files Created/Modified

- `build/x86_64/services-sweep.rc` — the D-02/D-08 functional-probe sweep (both venues' single source; 15-03 bakes it unchanged)
- `build/x86_64/services-termrc` — the services image's termrc override (staged for 15-03)
- `build/x86_64/svctcpecho.c` — the compiled one-shot loopback tcp client (read-a-line + connect-only)
- `sys/src/9k/k10/k10t1` — rootdir 43→58 (the sweep + 13 services members + the two tcpNNNN service programs)
- `sys/src/9k/k10/root/k10t1.rc` — the services leg (child-rc source after the tier-1 sweep)
- `sys/src/9k/port/devroot.c` — the standard /rc root dir (the srv -e enabler; NOT an Nbootfiles raise — 58 fits 64, A6 untriggered)
- `build/mm/build-9k10t1.sh` — the services staging (magic-checked copies + two host-side builds) + the build-tree lock + the 58-count assert
- `build/mm/boot-9k10t1-sweep.sh` — the SVC classification arm + the SVC-DONE poll + its build-tree lock
- `evidence/15-02-donor-services-serial.log` + `...-harness.log` — the retained final capture (the record 15-03 consumes)

## Decisions Made

See key-decisions in the frontmatter — the load-bearing ones: the ipinfo query route for cs, loopback-before-daemons (the netlist scan), the staged service-name copies under one listener, the /rc root dir (not a ceiling raise), the host-side link-only build lane, and the capture-as-evidence OBS-line discipline.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The donor's h*-only command vocabulary**
- **Found during:** Task 1 (first tracer boot)
- **Issue:** bare `test`/`sleep` resolve to nothing on the donor (only rc/echo/bind embed unprefixed — the second-name staged copies); every call was an rc "file does not exist" failure.
- **Fix:** the $B/h* prefix everywhere (fn bodies included); the header records the discipline.
- **Files modified:** build/x86_64/services-sweep.rc
- **Verification:** mntgen green from the next boot.
- **Committed in:** 1a43d5b16

**2. [Rule 1 - Bug] cs registers its networks at startup — the #I bind must precede the daemons**
- **Found during:** Task 1 (the SVC-CSOBS diagnosis: "/net/tcp/clone probehost" — the no-translation fallback)
- **Issue:** cs's netinit scans /net/*/clone once at startup; without #I bound, `tcp' is an unknown network and every query falls through.
- **Fix:** the loopback bring-up block moved ahead of cs (and every net-naming daemon); the 14-09 netip-loopback shape (raw ipifc ctl writes) adopted in the tracer rather than Task 2.
- **Files modified:** build/x86_64/services-sweep.rc
- **Verification:** the query answered from the ndb once bound.
- **Committed in:** cf2149c59

**3. [Rule 1 - Bug] Harvey rc's dialect: braced if-not bodies are parse errors (and a parse error kills the whole SOURCED file)**
- **Found during:** Task 1 (three "silent wedge" boots decoded by driving the serial console interactively)
- **Issue:** `if not {` is rejected by rc's parser; because rc parses each braced block whole before running it, the error aborted the sweep mid-file with NO error line on the capture — it looked exactly like a kernel wedge (the bogus wedge theory is retracted: the bind itself works, proven by typing it).
- **Fix:** flat status-variable ladders (`svclo`/`svcx` state + `if(~ $var state)`); the rc-syntax discipline recorded in the sweep header.
- **Files modified:** build/x86_64/services-sweep.rc
- **Verification:** the sweep ran to SVC-DONE from the next boot.
- **Committed in:** e636ce9f6

**4. [Rule 1 - Bug] The fd-dup redirect `<[0=7]` is unreliable in nested sourced blocks**
- **Found during:** Task 1 (the decoded "cat: write error copying <stdin>: unsupported ctl" — the dup landed reversed: a cat read the CONSOLE and wrote into the ctl channel)
- **Issue:** silent block under the no-input harness (only serial input could unmask it); the racy-looking behavior was this bug's dependence on context.
- **Fix:** the whole fd-hold machinery eliminated — the 14-09 fixture's own transient shape (clone read + per-write ctl opens; its measured finding: the binding persists across ctl closes).
- **Files modified:** build/x86_64/services-sweep.rc
- **Verification:** SVC-LCONV + SVC-LOOPBACK-UP stable from the next boot.
- **Committed in:** b53571214, 1e1a8aeef

**5. [Rule 1 - Bug] libndb drops an unterminated final line; cs's query forms**
- **Found during:** Task 1 (the env ndb's missing newline; the "can't translate service" empty-service rejection; the `=*ip` lexer token error)
- **Issue:** three independent probe-shape faults: rc env writes carry no trailing newline (Brdline drops the last tuple whole); a bare `tcp!host` query's empty service part fails ipserv; a bare `=' in a match pattern is a lexer token error that kills the sourced file.
- **Fix:** the value embeds its newline literally (closing quote on the next line); the query carries an explicit port — then moved to the measured ndb-backed `!ipinfo` route when the dns-first errstr swamping surfaced; '='-free patterns.
- **Files modified:** build/x86_64/services-sweep.rc
- **Verification:** SVC-CSOBS ip=127.0.0.1.
- **Committed in:** db34e2aea, bb0697080, 1af91fe7a, 4fc9764e1

**6. [Rule 2 - Missing critical] dns needs a non-loopback address (myipaddr never falls back)**
- **Found during:** Task 2 ("can't read my ip address" sysfatal with the loopback UP)
- **Issue:** libip's myipaddr skips 127.0.0.0/8 unconditionally ("isn't the friggin loopback address" — but no fallback either).
- **Fix:** the one loopback ipifc carries a second probe address (192.168.237.1) alongside 127.0.0.1.
- **Files modified:** build/x86_64/services-sweep.rc
- **Verification:** dns started and answered.
- **Committed in:** 4e870fcf8

**7. [Rule 2 - Missing critical] The venue could not run ANY srv -e service — /rc was absent from devroot's root**
- **Found during:** Task 2 (srv's exec'd child rc died sourcing the default /rc/lib/rcmain)
- **Issue:** srv -e hardcodes /bin/rc -c; rc sources /rc/lib/rcmain unconditionally; devroot's rootreset had no rc dir — nothing could bootstrap it (mount points must exist).
- **Fix:** devroot rootreset gained the standard `rc' root dir (raise-comment precedent) + the sweep bootstraps /rc/lib/rcmain namespace-style (mntgen /rc + bind -b /boot /rc/lib). NOT an Nbootfiles raise — 58 embedded files fit the 64 ceiling (A6 untriggered).
- **Files modified:** sys/src/9k/port/devroot.c, build/x86_64/services-sweep.rc
- **Verification:** SVC-OK srv + SVC-OK exportfs (sentinel byte-compared through the mount).
- **Committed in:** d423eb4ad

**8. [Rule 3 - Blocking] factotum's staging path**
- **Found during:** Task 3 (build assert: amd64/bin/factotum missing)
- **Issue:** the tier-1 leaf installs factotum to amd64/bin/auth/factotum, not the top level.
- **Fix:** svc_copy re-pointed.
- **Files modified:** build/mm/build-9k10t1.sh
- **Verification:** build green.
- **Committed in:** aa0e35fdf

**9. [Rule 1 - Bug] keyfs's getauthkey escalates into an interactive console prompt**
- **Found during:** Task 3 (the sweep blocked at "Password:" — readln opens /dev/cons BY PATH)
- **Issue:** the plan's projected "zero-key+warning" landing lives one call lower (inside readnvram); this tree's getauthkey prompts on failure — a no-input venue wedges.
- **Fix:** namespace isolation — the READ-ONLY tier-1 sentinel bound over /dev/cons for the keyfs run (the same fixed line feeds password AND confirm — offset 0 per fresh open; an rw env file FAILED first: the differing prompts rewrote the input, mismatch loop). The bind stays for the venue lifetime (nothing later opens /dev/cons fresh; no unmount binary embedded).
- **Files modified:** build/x86_64/services-sweep.rc
- **Verification:** "1 keys read" + the create path green.
- **Committed in:** 1998e9ccf, 42c96e231

**10. [Rule 1 - Bug] keyfs's write-back lands in the server's RFENVG-private env copy**
- **Found during:** Task 3 (the keyfile never materialized in the parent's /env)
- **Issue:** the server child forks RFENVG — its writeusers writes a private env copy.
- **Fix:** the honest observable is the USER DIRECTORY standing through the mount after hmkdir's DMDIR create (Create itself calls writeusers — the create IS the write-back exercise).
- **Files modified:** build/x86_64/services-sweep.rc
- **Verification:** SVC-OK keyfs; the full leg green.
- **Committed in:** 5fb06b007

**11. [Rule 2 - Missing critical] No in-tree one-shot tcp client fits the sweep**
- **Found during:** Task 2 design (dial/pass/expect are console- or expect-shaped; rc cannot hold a conv without the fd-dup machinery eliminated in #4)
- **Fix:** build/x86_64/svctcpecho.c (20-line fixed-input client, read-a-line + connect-only modes) + hmkdir, both host-side 6c/6l link-only builds in the staging phase (the nm-input.6 discipline; mkdir is the write-flow class, not a tier member on x64).
- **Files modified:** build/x86_64/svctcpecho.c, build/mm/build-9k10t1.sh
- **Verification:** SVC-OK listen + SVC-OK authsrv.
- **Committed in:** ebf06c533, 2c5be2b4c

---

**Total deviations:** 11 auto-fixed (7 bugs, 3 missing-critical/enabler, 1 blocking-path) across 13 fix commits. **Impact on plan:** every fix is a measured venue fact recorded in the sweep's header discipline notes (the file is both venues' single source — 15-03 inherits them for free); the two structural additions (the /rc root dir, the host-side build lane) are permanent venue capability. The terminal truths (all nine subjects interaction-backed, tier-1 regression green, probe semantics settled once) are met exactly as planned.

## Authentication Gates

None.

## Known Stubs

None — no placeholder paths, no unwired data, no skipped verifications. The one conditional SKIP block (k10t1-no-loopback) never fired: 9k devip loopback is proven live on this venue (the flagged assumption RESOLVED, not deferred). The D-08 boundary held: no p9sk1/authkey artifacts anywhere in the capture (verify 2A green on the strict arm).

## Issues Encountered

- The diagnostic arc: three boots presented as "silent kernel wedges" but were rc parse/redirect faults — the interactive serial session (qemu -serial stdio with scripted input) decoded all of them in two sessions; that rig is now the recorded pattern for serial-only venues.
- The plan's keyfs model ("readnvram's zero-key path serves the empty DB") was one call off: the zero-key path exists inside readnvram, but authcmdlib's getauthkey always escalates a failed read into an interactive prompt — the sentinel-bind isolation preserves the probe's substance (absent keyfile + throwaway key + write-back exercise) without touching key material rules.

## User Setup Required

None — no external service configuration required.

## Next Plan Readiness

READY for 15-03 (the r9 services leg): the sweep file + services-termrc bake unchanged; the probe semantics, the rc-dialect rules, the loopback bring-up (with the myipaddr second address), and the SVC OBS-line discipline are settled once for both venues; the harness's computed-expectation shape is the r9 gate's template. The ipconfig T1-SKIP flip (the 14-09 stale reason) rides the r9 leg per plan. The retained capture (evidence/15-02-donor-services-serial.log) is the record the r9 leg's marker parity compares against.

## Self-Check: PASSED

- files exist: services-sweep.rc, services-termrc, svctcpecho.c, both evidence captures, k10t1, k10t1.rc, devroot.c, both mm scripts ✓
- commits are ancestors of HEAD: 64506863b..5fb06b007 (25 commits, measured) ✓
- plan-level verification green at HEAD: the full donor leg exit 0 three consecutive times — T1-VENUE-PASS ok=36 skip=24 fail=0 svc=9/0 ok/skip fail=0 done=yes ✓

---
*Phase: 15-userspace-tiers-bootfs-payload-amd64*
*Completed: 2026-11-10*
