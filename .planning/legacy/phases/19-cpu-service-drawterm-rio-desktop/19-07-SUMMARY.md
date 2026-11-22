---
phase: 19-cpu-service-drawterm-rio-desktop
plan: "07"
subsystem: infra (mkimage §15 composition + the cpu bring-up gate)
tags: [cpurc, aux-listen, tcp17010, cpu-service, mkimage, mkpaqfs, p9sk1, expect, qemu, hostfwd]

# Dependency graph
requires:
  - phase: 19-cpu-service-drawterm-rio-desktop/19-01
    provides: the §15 seed contract + the append seam (the init=/amd64/init -c bootconf) + the service=cpu boot architecture
  - phase: 19-cpu-service-drawterm-rio-desktop/19-03
    provides: the kernel #! pre-pass + REMOVE/WSTAT/FWSTAT arms the service-file exec and §15 ride
  - phase: 19-cpu-service-drawterm-rio-desktop/19-05
    provides: flake.lock r9 pin 0a4b853 — every composed gate this plan ran consumes kernel-x64 from it
  - phase: 18-in-guest-p9sk1-auth-domain/18-04
    provides: the auth-suite tier entries, authdomain-gen, the 14f2b feed discipline, and the listen1 -t trusted-arm precedent
provides:
  - mkimage §15 as the FULL seventh contract — bootfs-cpu.paq (chain + the 18 auth suite h* set + cmd/cpu + aux/listen + the real #! cpurc + tcp17010 at both staged homes + the font payload + §15's own bake-time p9sk1 domain), 21499689 bytes
  - the committed cpurc (root-tree/rc/bin/cpurc) — the fail-loud ladder (pairing -> loopback -> keyfs -> listen1 -t authsrv -> factotum -> aux/listen -t /bin/service tcp) with the cpurc: marker vocabulary
  - the D-12 creds emission (authdomain-gen's auth-creds artifact + mkimage's host-side rm-before-cp copy to the gitignored out/auth-creds.txt with line-shape asserts — the 19-08 parse contract)
  - the draw-presence-guarded profile (DESK-02/DESK-05): the window-manager exec and the in-session plumber start sit behind test -e /mnt/term/dev/draw
  - build/x86_64/cpu-bringup.expect + the two green captures — 19-08's stage vocabulary and 19-09's --system substrate
affects: [19-08 (dials THIS image through hostfwd 17010/5670; reads out/auth-creds.txt), 19-09 (the --system=cpu boot app selects bootfs-cpu.paq), the r9-side q35cpu boot profile (its 19-03 fixture expectations are stale — see Deviations)]

actuals:
  tokens: 11115     # chars/4 over the realized production diff (44,460 chars across the 3 task commits; evidence logs excluded)
  tasks: 3
  commits: 3        # MEASURED: git rev-list --count 1b60b69d8..HEAD
plan_head_before: 1b60b69d865027126f2a98021411afe70edf9ae1
plan_head_after: 8cc749bcab6d9a39b5bc576f6801994175654a52

tech-stack:
  added: []          # zero packages — committed tree members only (T-19-SC honored)
  patterns:
    - "the seventh bake contract's shape: §14's computed h* resolver REUSED by reference (AUTH_HLIST) + the cpu additions at donor-conventional exec paths (/amd64/bin/cpu, /amd64/bin/aux/listen — the k10root.proto homes)"
    - "stage_payload's two-armed ndb compose: an optional authdom parameter keeps the 18-04 authdom row (asserted present) while the two-arg services arm stays byte-identical (strip asserted)"
    - "in-guest sed parses are GROUP-FREE by discipline: this lineage's sed silently no-ops \\( \\) capture substitutions (measured on plan9port's; proven identical plan9+GNU in the group-free form)"
    - "guest-side service-dir surfacing: the paq-local bin/service is shadowed by the namespace's plain /bin bind, so the image stages a rc/bin/service union copy (making the donor-style bind target resolve — r9 sys_bind needs an existing target) AND the paq copy"

key-files:
  created:
    - build/x86_64/root-tree/rc/bin/cpurc
    - build/x86_64/root-tree/bin/service/tcp17010
    - build/x86_64/cpu-bringup.expect
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-07-bringup-a.log
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-07-bringup-b.log
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-07-bringup-a-stages.txt
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-07-bringup-b-stages.txt
  modified:
    - build/x86_64/mkimage.sh            # §14f2c creds copy + stage_payload auth arm + §15 full contract
    - build/x86_64/authdomain-gen.c      # the auth-creds emission in generate()
    - build/x86_64/tiers/tier2-services.txt  # the cmd/cpu append
    - build/x86_64/root-tree/usr/glenda/lib/profile  # the draw guard + the in-session plumber

key-decisions:
  - "cpurc's factotum key write parses the authdom from the BAKED /lib/ndb/local (the ipnet row) with a GROUP-FREE sed chain (comment-delete, line-select, strip-to-token, strip-tail) — plan9port's sed silently no-ops \\( \\) capture substitutions (measured host-side BEFORE the first boot: the \\( \\) form returned empty), and the group-free form is byte-identical on plan9 and GNU sed"
  - "§15 stages §14's computed AUTH_HLIST by reference (one resolver, never a second list) plus its own additions; cpu rides at /amd64/bin/cpu (the #! interpreter path) and listen at /amd64/bin/aux/listen (the donor k10root.proto's own home — the cpurc line names it absolutely)"
  - "the service file stages at BOTH bin/service/tcp17010 (the Task-1 committed paq path, manifest-asserted) and rc/bin/service/tcp17010 (the union copy that makes cpurc's `bind /root/bin/service /bin/service' target RESOLVE — r9's sys_bind requires an existing target and the namespace's plain /bin bind shadows paq-local /bin/service, the 19-03 finding)"
  - "the 19-03 fixture legs (cpurc: RAN/SHEBANG-OK/REMOVE-OK) retired with the fixture body: the #! exec they proved is now load-bearing at the tcp17010 service point itself; the r9-side q35cpu boot profile's expectations re-key in the next r9-touching plan (this plan's authority is harvey-side only — no r9 commits)"
  - "the witness manifest re-pin reproduced the 38 pinned members byte-for-byte (digest UNCHANGED: 695dbf28b6c0a6a389bd068f4eff4261fafbc329b1ce5c13527bf1b3052195ba) — the fresh per-entry tier builds are deterministic; the re-run was the standing remedy's confirmation, not a correction"
  - "the probe's bake-manifest needles are parens/§-free substrings: a UTF-8 § inside a BRE \\( \\) group silently no-matches under expect's exec grep locale (measured: grep -c returned 0 for the parenthesized form while plain substrings match)"

patterns-established:
  - "Bake-log-as-env-contract: the probe asserts the §15 manifest lines host-side (exec grep over R9_BAKE_LOG) before trusting any spawn spend — the bake log joins the expect env contract"
  - "Emission-order stage discipline enforced by failure: the bootconf line streams between the banner and paqfs; a stage placed after paqfs waits forever (the 15-03 lesson re-learned on the first run — the timeout localized it exactly)"

requirements-completed: [CPU-02, CPU-03, DESK-02, DESK-05]

coverage:
  - id: D1
    description: "The cmd/cpu tier entry + witness re-pin + the D-12 creds emission + the tcp17010 service file"
    requirement: CPU-03
    verification:
      - kind: other
        ref: "TIER-CREDS-SVC-OK + CREDS-EMIT-WIRED (the plan's verify legs) + the full 9-contract bake green with out/auth-creds.txt at 444 carrying user=/password= (redacted in evidence) + the manifest digest 695dbf28… unchanged across the re-pin"
        status: pass
    human_judgment: false
  - id: D2
    description: "§15 as the full seventh contract: tier asserts, computed h* set, §15's own p9sk1 domain with the pairing assert over staged copies, feed + surviving creds copy, ndb local with the authdom row, the font payload via stage_payload's auth arm, the real cpurc + tcp17010 staged, KERNDATE/cap/magic checks, the manifest print"
    requirement: CPU-02
    verification:
      - kind: manual_procedural
        ref: "sh build/x86_64/mkimage.sh — '== PASS: baked …/bootfs-cpu.paq (21499689 bytes; chain + the auth suite + cmd/cpu + aux/listen + the real cpurc + tcp17010 + the font payload + the §15 p9sk1 domain…)' + the 28 'cpu manifest (§15):' lines + 'cpu pairing verified' + all 8 prior contracts PASS; three doctored-input negative controls red with named errors (template token dropped / cpurc marker dropped / hash-bang doctored to -N -R)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The committed cpurc: the six-stage fail-loud ladder with both listener shapes (listen1 -t for authsrv, aux/listen -t for the service dir), cs/secstored intentionally absent, the feed-sentinel factotum key write"
    requirement: CPU-02
    verification:
      - kind: e2e
        ref: "CPURC-SHAPE-OK (the plan's verify) + the live ladder: cpurc: PAIR-OK, KEYFS-OK, AUTHSRV-LISTEN, FACTOTUM-OK, LISTEN-17010, DONE — each exactly once, in order, in BOTH bring-up captures (evidence/19-07-bringup-{a,b}.log)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The in-session-only desktop: the profile's cpu case guards the window-manager exec on draw presence with plumber starting under the same guard; DESK-02's boot-side proof (zero font-line markers in the boot capture)"
    requirement: DESK-02
    verification:
      - kind: e2e
        ref: "PROFILE-GUARDS-OK (the plan's verify) + the D-17 shape asserts held (terminal case rio-free, exec rio + latin1.8.font still in the cpu case — stage_payload P6 green in every bake) + the whole-capture post-pass: latin1.8.font count 0 in both bring-up captures (an unguarded rio would print the font path in its startup failure and red exactly there)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The cpu-bringup probe, green twice consecutively at the advanced pin: bake -> ISO-with-append compose -> qemu under the slirp hostfwd argv -> the ladder asserted with zero FAIL/panic/font markers"
    requirement: CPU-03
    verification:
      - kind: e2e
        ref: "BRINGUP-A-GREEN + BRINGUP-GREEN-TWICE (the plan's verify legs): expect exit 0 twice, cpurc: DONE present in both captures, FAIL count 0, desktop font-line count 0, distinct per-bake domain fingerprints (dd5fb826… vs f40265f8… — genuinely independent runs); kernel from the 0a4b853 lock pin, KVM"
        status: pass
    human_judgment: false

duration: 24 min
completed: 2026-11-15
status: complete
---

# Phase 19 Plan 07: The Seventh Contract — cpu image composition + bring-up Summary

**mkimage §15 expanded to the full seventh contract (chain + the 18 auth suite + cmd/cpu + aux/listen + the real #! cpurc + tcp17010 at both staged homes + the font payload + §15's own bake-time p9sk1 domain), proven live by cpu-bringup.expect booting the image at the 0a4b853 pin through the append seam to the full cpurc ladder and the tcp17010 announce — green twice, with the desktop provably absent from boot**

## Performance

- **Duration:** 24 min (background builds overlapped Tasks 1-2)
- **Started:** 2026-11-15T02:19:02Z
- **Completed:** 2026-11-15T02:43:02Z
- **Tasks:** 3/3
- **Files modified:** 7 production + 4 evidence

## Accomplishments

- **Task 1:** cmd/cpu appended to tier2-services.txt (the per-entry gate is this plan's bring-up probe); the witness manifest re-pinned via build/mm/build-9k10t1.sh — the 38 members reproduced byte-for-byte (digest unchanged, 695dbf28…); authdomain-gen emits the D-12 creds artifact (user= then password=, 0444) in the same generate() run as the feed; mkimage §14f2c copies it rm-before-cp to the gitignored out/auth-creds.txt with line-shape asserts; the tcp17010 service file committed with the pinned authenticated hash-bang (#!/amd64/bin/cpu -R, 755).
- **Task 2:** §15 expanded from the 19-01/19-03 seed into the full contract — §14's computed auth-suite h* set reused by reference, cpu at /amd64/bin/cpu and listen at the donor's /amd64/bin/aux home, §15's OWN bake-time domain with the D-02 pairing asserted over the staged copies, the feed + the surviving creds copy, the font payload through stage_payload's new AUTH ndb arm (authdom row kept and asserted), the real cpurc + the service file at both staged homes, and the 14n/14o-class cap/magic checks; the profile gained the draw-presence guard and the in-session plumber.  Three doctored-input negative controls red with named errors.
- **Task 3:** cpu-bringup.expect on the auth-gate chassis — the §15 bake manifest asserted host-side, the staged boot in emission order (banner -> bootconf -> paqfs -> version), the hostfwd pair observed host-side (17010 + 5670), the cpurc ladder in order, the post-cpurc prompt, expect_before fast-fails (panic + cpurc: FAIL), and the whole-capture post-passes (zero FAIL, zero panic, zero desktop font-line markers).  Driven manually (bake -> compose with the init append -> the emitter's slirp argv -> expect); two consecutive green full cycles on KVM at the 0a4b853 pin.

## Task Commits

1. **Task 1: cmd/cpu tier entry + D-12 creds emission + tcp17010 service file** - `f97b4f6a0` (feat)
2. **Task 2: §15 full contract + the real cpurc + the in-session-only profile** - `ee51c9f40` (feat)
3. **Task 3: cpu-bringup probe, green twice + evidence** - `8cc749bca` (feat)

**Plan metadata:** (this SUMMARY + STATE/ROADMAP — the docs commit follows)

## Files Created/Modified

- `build/x86_64/tiers/tier2-services.txt` - the cmd/cpu append (append-only respected; exactly one new entry line)
- `build/x86_64/authdomain-gen.c` - the auth-creds artifact (user=/password=, 0444, the 19-08 parse contract)
- `build/x86_64/root-tree/bin/service/tcp17010` - the donor-shaped service file, pinned #!/amd64/bin/cpu -R, 755
- `build/x86_64/mkimage.sh` - §14f2c (creds copy + asserts), stage_payload's authdom arm, §15's full rewrite
- `build/x86_64/root-tree/rc/bin/cpurc` - the real cpurc: the fail-loud ladder + the marker vocabulary
- `build/x86_64/root-tree/usr/glenda/lib/profile` - the draw-presence guard + the in-session plumber (DESK-02/05)
- `build/x86_64/cpu-bringup.expect` - the bring-up probe
- `evidence/19-07-bringup-{a,b}.log` + `-stages.txt` - the two green captures + stage transcripts

## Decisions Made

See key-decisions. Additionally: cpurc's owner token is read from #c/user (whatever boot's glenda() path set — never hardcoded); the DEMO shared password makes the feed line the hostowner's (the gate is the DEMO path's, D-01 lineage); /mnt/term staged as a mount point (the donor proto's own dir — cpu -R's remoteside mounts the client exportfs there, 19-08's target); secstored/cs intentionally not started (numeric dials only — cs's $attr translation reds on this venue, 18-05).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The authdom parse's \( \) capture groups silently no-op on this sed lineage**
- **Found during:** Task 2 (host-side pre-boot validation of the cpurc's sed shape)
- **Issue:** The drafted `s/.*authdom=\([^ ]*\).*/\1/p` extraction returns EMPTY under plan9port's sed (the closest cousin of the guest binary): substitutions carrying \( \) capture groups silently fail to apply (measured; GNU sed works — a lineage divergence that would have red the factotum leg in-guest with no hint why)
- **Fix:** The group-free chain `/^#/d; /authdom=/!d; s/.*authdom=//; s/ .*//; p` — proven byte-identical on plan9 and GNU sed; the finding recorded in cpurc's comment and the §15 bake ran green with it live
- **Files modified:** build/x86_64/root-tree/rc/bin/cpurc
- **Verification:** Both bring-up captures show cpurc: FACTOTUM-OK (the parse fed the key write)
- **Committed in:** ee51c9f40

**2. [Rule 3 - Blocking] stage_payload needed a second ndb arm**
- **Found during:** Task 2 (the §15 compose)
- **Issue:** The plan requires BOTH the font payload via stage_payload AND an authdom-bearing /lib/ndb/local (cpurc parses it) — but the committed function strips the authdom row and asserts its absence (the services image's no-domain contract); §15 could not declare the payload as-is
- **Fix:** An optional third parameter (authdom): the auth arm substitutes @AUTHDOM@ from the caller's own generator run and asserts the row present; the two-arg services arm is byte-identical to the 15-08 shape (its strip asserts unchanged; all 9 contracts bake green; a doctored-template negative control reds the new arm's assert)
- **Files modified:** build/x86_64/mkimage.sh
- **Verification:** bake2/bake3 logs — the cpu arm prints "auth arm, row KEPT", the services arm prints the strip line unchanged; neg1 red
- **Committed in:** ee51c9f40

**3. [Rule 1 - Bug] The probe's bootconf stage was ordered after paqfs**
- **Found during:** Task 3 (run A's first attempt: TIMEOUT at the bootconf stage after 420s)
- **Issue:** The kernel prints `devenv: bootconf` between the banner and paqfs — a stage placed after the paqfs stage waits forever on text the earlier match already consumed (the 15-03 emission-order lesson; the boot itself was fully green in that capture, including the whole ladder)
- **Fix:** The stages reordered to true emission order (banner -> bootconf -> paqfs -> version), with the lesson documented at the stage block
- **Files modified:** build/x86_64/cpu-bringup.expect
- **Verification:** Run A re-driven end-to-end green (exit 0, ALL STAGES PASS), then run B green
- **Committed in:** 8cc749bca

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 blocking)
**Impact on plan:** All three were prerequisites for the plan's own verifies passing; no scope creep. The sed-lineage and grep-locale findings are recorded as venue facts (key-decisions).

## Issues Encountered

- The r9-side q35cpu boot profile (cargo xtask boot-test --config q35cpu) is now STALE by design: its stage machine expects the retired 19-03 fixture markers (cpurc: RAN, SHEBANG-OK, REMOVE-OK) and its config carries no ip dev, so it cannot boot the real cpurc's ladder either. It is not in hosted CI (the three CI profiles are q35/q35lowmcfg/q35net) and no harvey gate runs it. The re-key (or retirement) belongs to the next r9-touching plan — recorded in §15's section comment and logged to the windows ledger.
- `expect` is not on the bare host PATH (devshell/nix-store only) — the manual driver used the store binary directly (a venue note for 19-08's driver, which rides the flake app's runtimeInputs and is unaffected).
- The first run A attempt was killed by the outer timeout at the misordered bootconf stage (deviation 3) — the boot itself was already fully green in that capture, so the fix was probe-side only.

## Known Stubs

None - no stubs, placeholders, or unwired data paths were introduced. (cs/secstored being unstarted is the plan's own recorded discretion — the binaries ride the h* payload unexecuted; the -G/desktop consumers are 19-08/19-09's, per the plan's own boundary.)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 19-08 dials EXACTLY this image (bootfs-cpu.paq at the 0a4b853 pin) through the committed hostfwd pair; its creds read is out/auth-creds.txt (§15's run is the surviving copy — asserted shape user=/password=); cpu-bringup.expect is its boot-ladder chassis (the plan's read_first names it)
- 19-09's --system=cpu boot mode selects bootfs-cpu.paq at the boot app's image seam with the same --append= compose
- The kernel bytes are untouched (no r9 commits this plan) — the FOD hash does NOT re-stale; the next r9-byte plan owns the next FOD pass
- The r9-side q35cpu profile re-key remains open (see Issues) — it must not block 19-08/19-09, which consume the harvey-side probe

## Self-Check: PASSED

- Created files exist: build/x86_64/root-tree/rc/bin/cpurc, build/x86_64/root-tree/bin/service/tcp17010, build/x86_64/cpu-bringup.expect, the four evidence files FOUND
- Task commits exist: f97b4f6a0, ee51c9f40, 8cc749bca all ancestors of HEAD (8cc749bca)
- Plan verify legs re-run at close-out: TIER-CREDS-SVC-OK, CREDS-EMIT-WIRED, CPURC-SHAPE-OK, PROFILE-GUARDS-OK, BRINGUP-A-GREEN, BRINGUP-GREEN-TWICE; acceptance criteria all pass (coverage block)

---
*Phase: 19-cpu-service-drawterm-rio-desktop*
*Completed: 2026-11-15*
