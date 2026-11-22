---
phase: 18-in-guest-p9sk1-auth-domain
plan: 04
subsystem: auth (p9sk1 domain image, mkimage sixth contract, listener topology, ndb seam, expect gate)
tags: [p9sk1, authdom, keyfs, authsrv, listen1, secstored, cs, factotum, nvram, ndb, netaddr-seam, mkimage, bootfs-auth, expect-gate, flake-app, AUTH-01, AUTH-04, AUTH-05, AUTH-07]

requires:
  - phase: 18-in-guest-p9sk1-auth-domain
    provides: 18-03's authdomain-gen (-o/-V/-i, the paired unit, the vendored-donor venue) and 18-02's pin b2a6151 (the kernel every gate boots)
provides:
  - "mkimage §14 (the sixth contract): bootfs-auth.paq — chain tree + the auth-suite set computed from tier data + authdomain-gen invoked AT BAKE with the D-02 pairing assert over the staged copies (AUTHBAKE_DOCTOR teeth seam, refusal proven twice)"
  - "the auth image's bring-up path: auth-termrc + auth-sweep.rc — the D-02 BOOT pairing check before keyfs, non-interactive keyfs over nvram= (N keys read), loopback, listen1 -t tcp!*!567 /amd64/bin/authsrv -d, secstored -v on 5356, cs over the baked default /lib/ndb/local, factotum with norbert's p9sk1 key"
  - "AUTH-05's seam staging: local.template's ether/@AUTHDOM@ extension, netaddr.rc's ether key (the single home), the 52:54:00 venue-literal grep class, the services-path auth-row strip (both ways asserted)"
  - "/boot/authfeed.txt — the generator's own last-user password, single newline-terminated line, the 18-05 sentinel-feed substrate"
  - "the flake app boot-r9-auth-x64 (authSmokeR9X64Pkg — the netSmoke shape + stdenv.cc) and its gate smoke-r9-auth.expect: green twice with both captures retained"
affects: [18-05 probe legs (auth/debug + speaksfor + secstore round-trip ride this image), Phase 19 drawterm (dials the hostfwd-5670 seam onto the authsrv listener), every future Nvrsafe guest consumer (the 112-byte ABI fact)]

actuals:
  tokens: 25047   # chars/4 over the realized production diff (cbd7da41b..abb879055: 100188 chars, 13 files, +1789/-26; estimate was 44000 — the same over-projection family)
  tasks: 3
  commits: 3      # ca9e687e7, 518e09559, abb879055 (measured: git rev-list cbd7da41b..HEAD)
plan_head_before: cbd7da41b9e6f9148b1f786e254479596436d0aa

tech-stack:
  added: []        # no new packages: stdenv.cc joined the AUTH app's runtimeInputs only (nixpkgs toolchain, the generator's host build — T-18-SC kept)
  patterns:
    - "generator-invoked-at-bake: §14 compiles authdomain-gen (host cc over the vendored donors) and runs it inside the bake — the domain material and the image can never diverge"
    - "the AUTHBAKE_DOCTOR teeth seam: an env-named byte-flip on the staged copy's scratch verify twin makes the bake's own refusal path runnable end-to-end (`AUTHBAKE_DOCTOR=nvram sh build/x86_64/mkimage.sh` exits 1 at the pairing assert)"
    - "two-consumer token templates: shared template + per-consumer substitution/strip with both-way asserts (the auth path substitutes all five tokens, the services path strips the domain row it cannot honestly carry)"
    - "timing-tolerant syslog post-pass: the ANNOUNCE line is asserted over the whole capture (message token, never the date prefix), never as a live expect stage — the 15-03 emission-order lesson applied"

key-files:
  created:
    - build/x86_64/auth-termrc
    - build/x86_64/auth-sweep.rc
    - build/x86_64/smoke-r9-auth.expect
    - nix/smoke-r9-auth-x64.sh
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-04-auth-a.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-04-auth-b.log
  modified:
    - build/x86_64/mkimage.sh        # §14 the AUTH bake + P2b's ether/auth-row handling + the 52:54:00 seam grep class
    - build/x86_64/authdomain-gen.c  # 112-byte nvram (the 6c guest ABI) + the feed artifact
    - build/x86_64/authdomain-selftest.sh
    - build/x86_64/tiers/tier2-services.txt   # the cmd/aux:listen1 leaf
    - build/x86_64/root-tree/lib/ndb/local.template
    - build/x86_64/root-tree/lib/netaddr.rc   # the ether key
    - flake.nix                       # authSmokeR9X64Pkg + the boot-r9-auth-x64 app

key-decisions:
  - "§14, not §13: the plan's auth-contract section number predates 18-01's substrate landing (the wave-1 numbering note); the AUTH contract is §14 on disk and navigates by name — executed as the STATE note pinned"
  - "The nvram FILE is 112 bytes: Harvey's amd64 6c END-PADS the all-char Nvrsafe to 112 while host compilers see 109 — the guest's readnvram reads sizeof bytes and a 109-byte file failed its length check SILENTLY (read 109 want 112, measured live; all five sums green over the donor-layout bytes, so member offsets are identical and only the tail pad differs). The generator emits 109-layout + 3 zero tail bytes; verify/-i/self-test re-pinned. Load-bearing for EVERY future guest consumer of Nvrsafe"
  - "The D-02 bake arm verifies the STAGED COPIES (a scratch twin assembled from the staged tree + the manifest, then authdomain-gen -V) — not the generator's output dir, so a staging-layer corruption trips the bake itself"
  - "@AUTHDOM@ substitutes from the generator run's own -i output, never from the seam: the domain is per-bake paired material and a seam key would be a second source violating D-02's pairing; the services bake STRIPS the auth row (its image has no domain — an authdom row without keys behind it would be a lie), asserted both ways"
  - "listen1 -t is the TRUSTED arm (listen1.c read in full, A5 resolved): it skips becomenone() — whose #c/user write + newns(\"none\") would strip the namespace authsrv needs (/mnt/keys, /lib/ndb/auth); without -v listen1 parks stdout on /dev/null — fine, AUTH-07 rides authsrv -d's syslog lines through the #c/cons fallback"
  - "The auth image carries exportfs/srv BINARIES (the plan's own staging list) but starts NO exportfs/listen services — D-11's minimality governs the SERVICE topology (exactly 567+5356), the binaries are inert payload"
  - "hsvctcpecho runs BEFORE cs exists — dial's no-connection-server path calls the clone directly (dial.c read), so the listener leg does not depend on the cs leg"
  - "The feed artifact is the LAST user's password (D-12's plain probe user — DEMO's norbert): the generator's manifest deliberately carries no passwords; the feed is the same run's convenience copy, staged read-only at /boot/authfeed.txt"

patterns-established:
  - "Pattern: guest-ABI-sized binary artifacts — when a host tool emits a binary consumed by guest code compiled with a DIFFERENT compiler, the artifact size is the GUEST's sizeof, not the host's; measure at the first live consumer, document at the emit site"
  - "Pattern: runnable bake-refusal teeth — an env-named doctor seam on the staged copies makes the bake's negative control a one-command rerun, not a one-time demonstration"
  - "Pattern: two-consumer ndb template — one committed template, per-consumer token substitution with strip-both-ways asserts, prose de-tokenized (the zero-leftover grep scans comments — the 15-07 lesson, self-inflicted twice in this plan)"

requirements-completed: [AUTH-01, AUTH-04, AUTH-05, AUTH-07]   # per the plan frontmatter; the shared-ID gate's verdict governs what REQUIREMENTS.md checks (AUTH-01 may be held for 18-05 — noted below)

coverage:
  - id: D1
    description: "mkimage §14 (the sixth contract) bakes bootfs-auth.paq: chain tree + the auth-suite set computed from the tier data + the generator invoked at bake"
    requirement: AUTH-01
    verification:
      - kind: integration
        ref: "sh build/x86_64/mkimage.sh → `== PASS: baked .../bootfs-auth.paq (2474777 bytes ...)` with `auth manifest:` rows for /adm/nvram, /adm/keys, /adm/domain.manifest, /lib/ndb/auth, /lib/ndb/local, /boot/authfeed.txt and 25 h* binaries"
        status: pass
    human_judgment: false
  - id: D2
    description: "The D-02 bake-arm pairing assert over the STAGED copies with proven teeth — a doctored staged copy refuses the bake"
    requirement: AUTH-01
    verification:
      - kind: integration
        ref: "AUTHBAKE_DOCTOR=nvram sh build/x86_64/mkimage.sh → exit 1 at `auth bake: the D-02 pairing assert REFUSED the staged copies` (verify: nvram: sha1 mismatch quoted) — proven after Task 1 AND re-proven with the final 112-byte code"
        status: pass
    human_judgment: false
  - id: D3
    description: "The D-02 BOOT pairing check before keyfs, printing the D-04 label banner; any mismatch names the artifact and skips every daemon leg (never a silent interactive wedge)"
    requirement: AUTH-04
    verification:
      - kind: e2e
        ref: "both evidence captures line 80: `authdom: PAIRED DEMO`; the pairing leg recomputes sha1 over /adm/nvram + /adm/keys + /lib/ndb/auth against /adm/domain.manifest (in-guest hsha1sum, flat rc ladder)"
        status: pass
    human_judgment: false
  - id: D4
    description: "keyfs starts NON-INTERACTIVELY over the baked nvram (nvram= plain assignment before the keyfs line, no -p) and prints its own N-keys-read line"
    requirement: AUTH-04
    verification:
      - kind: e2e
        ref: "both captures: `3 keys read` then `AUTH-OK keyfs` (/mnt/keys mounted inside a bounded spin); the exact getpass string `can't read NVRAM, please enter machine key` PROVABLY absent (whole-capture post-pass, zero occurrences)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The listener topology: listen1 -t tcp!*!567 /amd64/bin/authsrv -d (flags ride via exec; -t skips becomenone) + secstored -v announcing tcp!*!5356, connect-accept proven in-guest, hostfwd 5670 observed host-side"
    requirement: AUTH-04
    verification:
      - kind: e2e
        ref: "both captures: `AUTH-LISTENOBS connected` → `AUTH-OK authsrv-listen`, `AUTH-OK secstored`; the gate's hostfwd_stage 5670 (exec ss -tln) stage-ok line; tier leaf `cmd/aux:listen1` (T2-ART listen1 e2fa59f6)"
        status: pass
    human_judgment: false
  - id: D6
    description: "AUTH-05 ndb staging through the seam: template tokens (@ETHER@, @AUTHDOM@), netaddr.rc's ether key as the single home, the venue-literal grep extended (52:54:00 class), the services-path auth-row strip"
    requirement: AUTH-05
    verification:
      - kind: integration
        ref: "bake log: `== auth ndb: /lib/ndb/local composed ... @AUTHDOM@=DEMO-harvey from the generator run`; the services P2b line carries `the 18-04 auth row stripped`; seam greps over root-tree/lib: 0/0 (clean) with the doctored control red (addr-hits=1 mac-hits=1 on a spiked copy)"
        status: pass
    human_judgment: false
  - id: D7
    description: "cs ANSWERS authdom->auth from the baked default /lib/ndb/local over loopback /net — AUTH-05's live proof (a real client interaction, never file-presence)"
    requirement: AUTH-05
    verification:
      - kind: e2e
        ref: "both captures line 162: `AUTH-CSOBS ipnet=harveynet ipmask=255.255.255.0 ipgw=10.0.2.2 authdom=DEMO-harvey auth=127.0.0.1` → `AUTH-OK cs` (hcsquery '!authdom=DEMO-harvey auth=*' through /net/cs; cs runs DEFAULT db, no -f — OQ3)"
        status: pass
    human_judgment: false
  - id: D8
    description: "The factotum substrate + the baked sentinel feed: norbert's p9sk1 key written from /boot/authfeed.txt with ctl read-back"
    requirement: AUTH-05
    verification:
      - kind: e2e
        ref: "both captures: `AUTH-FACTOBS key proto=p9sk1 user=norbert dom=DEMO-harvey !password?` (factotum masks the secret — the proto/user/dom trio is the observable) → `AUTH-OK factotum`; /boot/authfeed.txt single-line + newline-terminated (bake-asserted)"
        status: pass
    human_judgment: false
  - id: D9
    description: "The app-per-gate expect gate green twice consecutively on KVM with both captures retained"
    requirement: AUTH-07
    verification:
      - kind: e2e
        ref: "AUTH-GATE-GREEN-TWICE (the plan's exact verify: two full `nix run .#boot-r9-auth-x64` runs; evidence/18-04-auth-{a,b}.log both carry the full marker chain, zero AUTH-FAIL, zero nvram-prompt)"
        status: pass
    human_judgment: false
  - id: D10
    description: "AUTH-07 day-one serial visibility: pairing banner + N keys read + ANNOUNCE/startup markers on every boot (daemon-verbose flags armed; syslog's #c/cons fallback is the mechanism — no writable /sys/log ever staged, asserted at 14l)"
    requirement: AUTH-07
    verification:
      - kind: e2e
        ref: "both captures carry ` ANNOUNCE /net/tcp!*!5356` (the syslog date-prefixed line through the console fallback; the gate asserts the message token over the whole capture); the 14-06 lineage post-passes zero AUTH-FAIL"
        status: pass
      - kind: integration
        ref: "mkimage 14l refuses a staged /sys/log tree (Pitfall 6 — the fallback IS the serial mechanism)"
        status: pass
    human_judgment: false

duration: 72min
completed: 2026-11-14
status: complete
---

# Phase 18 Plan 04: Auth Domain Vehicle (Sixth Contract + Gate) Summary

**The sixth mkimage contract (§14) bakes bootfs-auth.paq over the 18-03 generator with the D-02 pairing asserted at bake AND re-verified at boot before a prompt-free keyfs, authsrv answers tcp!*!567 via listen1 -t with -d armed, secstored announces 5356, cs answers authdom from the seam-pure baked local, and factotum carries norbert's p9sk1 key — one path generator-to-serial, green twice.**

## Performance
- **Duration:** 72 min - **Started:** 2026-11-14T03:58Z - **Completed:** 2026-11-14T05:11Z - **Tasks:** 3/3 - **Files:** 13 (+1789/−26 over cbd7da41b)

## Accomplishments
- AUTH-01's bake arm, AUTH-04's vehicle, AUTH-05's seam staging, and AUTH-07's substrate are live: the auth domain is a bootable, observable image whose Phase-19 consumer (18-05's probes + drawterm) dials exactly this listener through the pre-wired hostfwd seam (observed host-side while the guest runs)
- The pairing is proven at BOTH ends of the bake→boot path: the bake refuses a doctored staged copy (exit 1, twice-proven via the committed AUTHBAKE_DOCTOR seam), and the guest re-verifies all three artifact digests against the manifest before any daemon starts — the nvram getpass prompt is PROVABLY unreachable on the green path (zero occurrences in both captures)
- The AUTH-05 content is seam-pure end to end: the ether MAC's single home is netaddr.rc, the domain token substitutes from the generator's own run, the venue-literal grep gained the 52:54:00 class, and the services image (the same template's other consumer) strips the auth row it cannot honestly carry — asserted both ways
- The load-bearing discovery of the plan: **Harvey's amd64 6c end-pads Nvrsafe to 112 bytes** (host compilers see 109) — the guest's readnvram reads sizeof bytes and the 109-byte artifact failed its length check SILENTLY, escalating straight to the interactive machine-key prompt (the exact T-18-08 wedge class, caught by the gate's never-appears assert on the first boot). Measured live with a temporary in-guest prober (read 109 want 112, all five sums green over the donor-layout bytes — member offsets identical, tail pad only); fixed at the generator (109-layout + 3 zero tail bytes), re-pinned in verify/-i/the committed self-test
- Green twice consecutively on KVM with both captures retained (evidence/18-04-auth-{a,b}.log); the 14-08 witness manifest NOT tripped by the listen1 tier leaf (a new /amd64/bin member — all 38 listed members still match, no re-pin); the r9 repo untouched at b2a6151 == origin/main; eqn.c and the other pre-existing dirty files untouched

## Task Commits
1. **Task 1: Sixth contract + bring-up to keyfs — generator-to-serial one path (tracer)** - `ca9e687e7` (feat; tracer feedback gate re-run end-to-end after commit: AUTH-TRACER-GREEN on the full pipeline)
2. **Task 2: Listener topology — listen1 leaf, authsrv -d on tcp!*!567, secstored -v on 5356** - `518e09559` (feat; AUTH-LISTENERS-GREEN + PROGRAM-EVAL-OK)
3. **Task 3: AUTH-05 ndb staging through the seam + cs answer + factotum substrate + green twice** - `abb879055` (feat; AUTH-GATE-GREEN-TWICE)

**Plan metadata:** this SUMMARY + STATE/ROADMAP/REQUIREMENTS (the docs commit follows)

## Files Created/Modified
- `build/x86_64/mkimage.sh` — §14 the AUTH bake (input asserts, the tier-computed staging resolver with the flat-TARG class, the generator at bake, the staged-copy pairing verify + AUTHBAKE_DOCTOR teeth, hsvctcpecho 6c/6l, manifests, KERNDATE/mkpaqfs-cap/magic) + P2b's ether substitution + auth-row strip + the 10b 52:54:00 seam class
- `build/x86_64/auth-termrc` / `build/x86_64/auth-sweep.rc` — the six legs: pairing → mntgen+nvram=+keyfs → loopback → listeners → cs → factotum → AUTH-DONE (flat rc ladders, the 15-02 measured shapes only)
- `build/x86_64/smoke-r9-auth.expect` / `nix/smoke-r9-auth-x64.sh` / `flake.nix` — the gate, the driver (r9x64auth- tag, auth-r9.log, the emitter's slirp argv with the D-04 hostfwd pair), authSmokeR9X64Pkg + the boot-r9-auth-x64 app
- `build/x86_64/authdomain-gen.c` / `authdomain-selftest.sh` — the 112-byte nvram + the feed artifact (GEN-SELFTEST-OK re-proven)
- `build/x86_64/tiers/tier2-services.txt` — the cmd/aux:listen1 leaf (append-only)
- `build/x86_64/root-tree/lib/ndb/local.template` / `lib/netaddr.rc` — the AUTH-05 rows + the ether key
- `evidence/18-04-auth-{a,b}.log` — the green-twice captures

## Decisions Made
- See key-decisions: the §14 numbering (the wave-1 note executed), the 112-byte guest-ABI nvram, the staged-copy bake arm, the generator-sourced @AUTHDOM@ + the services-path strip, listen1 -t's trusted semantics (A5), binaries-vs-services on D-11, the no-cs dial fallback, the last-user feed convention
- The expect gate's AUTH-FAIL fast-fail arm (expect_before) localizes a failed leg in seconds instead of a 600s stage timeout — the sweep's `AUTH-FAIL sweep` terminal marker guarantees a fast-failing negative terminal on any partial failure
- The ANNOUNCE assertion is a whole-capture post-pass (timing-tolerant), never a live stage — secstored's syslog line can land before any stage that would wait for it (the 15-03 emission-order lesson)

## Deviations from Plan

**1. [Rule 1 - Bug] The nvram artifact must be 112 bytes — the amd64 guest compiler end-pads Nvrsafe**
- **Found during:** Task 1 (the first tracer boot wedged at `Password:` behind `can't read NVRAM, please enter machine key`)
- **Issue:** Harvey's 6c computes sizeof(Nvrsafe)==112 for the all-char struct (end-padded to 8); host compilers compute 109. The guest's readnvram reads sizeof bytes; the 109-byte artifact returned 109 != 112 — the SILENT err path under flag 0 (no diagnostic anywhere), escalating straight to getauthkey's interactive prompt. 18-03's generator/self-test were host-verified only; the first guest consumer exposed the ABI delta.
- **Fix:** The generator emits the 109-byte layout + 3 zero tail bytes (member offsets proven identical in-guest — a temporary 6c-compiled prober printed `read 109 want 112` with all five sums green); verify/-i re-pinned to Nvramfile=112; the self-test's byte assert updated; mkimage §14's staged-size assert updated. The prober was removed before the Task-1 commit.
- **Files modified:** build/x86_64/authdomain-gen.c, build/x86_64/authdomain-selftest.sh, build/x86_64/mkimage.sh
- **Verification:** GEN-SELFTEST-OK re-proven; the next boot printed `3 keys read` and the gate went green end-to-end
- **Commit:** ca9e687e7

**2. [Rule 3 - Blocking] The generator gained a `feed` artifact (the baked sentinel feed's source)**
- **Found during:** Task 3 design
- **Issue:** The plan requires /boot/authfeed.txt staged "with the DEMO-shared password from the same generator run", but the generator emits no password anywhere (the manifest deliberately carries none — 18-03's design), and passwords cannot be re-derived from the keys file (passtokey is one-way).
- **Fix:** generate() writes `feed` = the LAST user's password + newline (D-12's plain probe user — norbert on the DEMO path); NOT a manifest member (the crypto pairing is untouched); the bake asserts single-line + newline and stages it read-only; the self-test pins its shape.
- **Files modified:** build/x86_64/authdomain-gen.c, build/x86_64/authdomain-selftest.sh, build/x86_64/mkimage.sh
- **Verification:** GEN-SELFTEST-OK; both green captures' `AUTH-OK factotum` (the key written from the feed)
- **Commit:** abb879055

**3. [Rule 3 - Blocking] The loopback bring-up landed in Task 2, not Task 3**
- **Found during:** Task 2 implementation
- **Issue:** The plan's Task-2 action (the listeners) omits the #I/ipifc bring-up explicitly (its read_first maps the loopback block to Task 3's cs leg), but listen1's announce and secstored's announce REQUIRE the /net tree — Task 2's own verify (AUTH-OK authsrv-listen) is unachievable without it.
- **Fix:** The services sweep's bring-up block (measured shape verbatim) precedes the listeners as the sweep's leg 3; Task 3's cs leg reuses the up state.
- **Files modified:** build/x86_64/auth-sweep.rc
- **Verification:** AUTH-LOOPBACK-UP in both captures before the listener verdicts
- **Commit:** 518e09559

**4. [Rule 3 - Blocking] The services bake (P2b) needed explicit handling of the two-consumer template**
- **Found during:** Task 3 implementation (the first rebake redded P2b's zero-leftover-token assert)
- **Issue:** The plan adds @AUTHDOM@-bearing rows to local.template ("additive rows only") but the services bake consumes the SAME template with a zero-leftover-@ assert — a domain token it cannot substitute (its image has no domain) would red every services bake.
- **Fix:** P2b substitutes @ETHER@ from the seam (the value's single home) and STRIPS the auth row before substitution (identified by its unsubstituted token, never by line number), asserted both ways: zero authdom= DATA rows and zero leftover tokens. The template's own prose documents the split (and itself had to be de-tokenized twice — the 15-07 comment-prose lesson, self-inflicted: the prose mentioned the AUTHDOM token and a bare at-sign, both caught by the bake's own assert).
- **Files modified:** build/x86_64/mkimage.sh, build/x86_64/root-tree/lib/ndb/local.template
- **Verification:** the services bake's P2b line now reads `... + ether; the 18-04 auth row stripped — no domain in this image`; all six bakes green in the final run
- **Commit:** abb879055

**5. [Rule 1 - Bug] The tier-2 resolver needed the flat-install (multi-word TARG) class**
- **Found during:** Task 1's first bake (hsecstored missing from the computed set)
- **Issue:** 11b's resolver stages a dir entry's whole $BIN/<rel> TREE — but cmd/auth/secstore installs FLAT (TARG = secstore secstored secuser aescbc to /amd64/bin/auth): the entry resolves to a file and only hsecstore was staged.
- **Fix:** For a dir entry whose mkfile declares a MULTI-word TARG, the resolver stages each TARG member from the install parent (the mkfile is the authority on what the entry builds); single-word TARGs keep the plain file rule (cmd/exportfs installs alone at the bin root).
- **Files modified:** build/x86_64/mkimage.sh
- **Verification:** the auth manifest counts the secstore members (hsecstore hsecstored hsecuser haescbc all staged; 25 h* binaries total)
- **Commit:** ca9e687e7

**6. [Documentation] The section numbering (§14, not the plan's §13)**
- The wave-1 STATE note executed as pinned: 18-01's substrate bake is §13 on disk, so the auth contract is §14; the plan's own line references (e.g. "§13k host-side 6c/6l build" = the services 11k block) were reconciled against the on-disk file before any edit. Recorded here per the dispatch's instruction.

**Total deviations:** 6 (2 Rule-1 bugs, 3 Rule-3 blockers, 1 documentation). **Impact:** deviation 1 is the plan's most consequential find — the 6c struct-pad ABI fact is load-bearing for every future guest consumer of Nvrsafe and would have surfaced as an opaque interactive wedge in any later plan; everything else is integration plumbing the first live bake/boot exposed.

## Issues Encountered
- The first tracer boot wedged at the nvram getpass prompt (deviation 1) — the gate's never-appears contract + the temporary in-guest prober localized it to the silent read-length mismatch in one diagnostic boot
- Two background gate runs collided on the hostfwd 5670 bind (a survivor qemu from a killed run held 127.0.0.1:5670 — qemu aborts with `Could not set up host forwarding rule`); resolved by sweeping the r9x64auth-tagged survivors; the per-run tag + EXIT-trap discipline then held for every subsequent run
- The 14-08 witness manifest was NOT tripped by the listen1 tier leaf (a new /amd64/bin member; the 38 listed members all still match — the per-member digest check is the witness's shape); no re-pin needed, recorded per the plan's instruction
- boot-r9-services-x64 was NOT run (stays blocked by the pre-existing #68 dns-stage kernel trap — STATE 17-06/17-07, A/B-identical pre-existing, not this plan's regression; this plan adds no services-image semantics beyond the template strip, whose bake leg is green)

## Authentication Gates
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Ready for 18-05 (wave 4, the phase's close): the image, the daemons, the cs answer, factotum's key, and the /boot/authfeed.txt sentinel feed are the probe substrate — auth/debug (leg 1), the speaksfor probe client + /lib/ndb/auth's negative control (leg 2), and the secstore round-trip (D-14) ride exactly this vehicle; the p9sk1 exchange over tcp!*!567 is what closes AUTH-06 and AUTH-07's per-ticket lines (this plan's flagged assumptions: AUTH-04/AUTH-07 complete at 18-05's exchanges by design, nothing silently claimed)
- Phase 19 note: drawterm dials host 127.0.0.1:5670 — the hostfwd pair is observed live by this gate, and the listener it lands on (authsrv -d via listen1) answers
- The 112-byte Nvrsafe fact is recorded in STATE for every future bake-side emitter; the AUTHBAKE_DOCTOR seam is the standing re-runnable proof of the bake's refusal path

Coverage contract note: every deliverable above carries automated verification with pass statuses from the committed gate/bake runs; no human-judgment deliverables. requirements-completed lists the plan frontmatter's IDs verbatim; the shared-ID gate's verdict on marking: AUTH-01, AUTH-04, AUTH-05 marked Complete (write_set_complete: true), **AUTH-07 held for 18-05** — the per-ticket tr-ok/tr-fail serial lines are 18-05's real exchanges (this plan's flagged assumption), and the hold is the gate working as designed: this SUMMARY remains AUTH-07's substrate evidence (D10).

## Self-Check: PASSED

All key files exist on disk (auth-termrc, auth-sweep.rc, smoke-r9-auth.expect, smoke-r9-auth-x64.sh, both evidence captures, this SUMMARY); all three task commits verified as ancestors of HEAD (ca9e687e7, 518e09559, abb879055); ../r9 clean at b2a6151 == origin/main; eqn.c's pre-existing working-tree dirt untouched by any commit.
